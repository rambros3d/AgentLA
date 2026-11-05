//! Serving of uploaded app files out of LittleFS and the streaming tarball
//! installer endpoints.
//!
//! Picoserve 0.20 API facts used here (verified against the 0.20.1 source):
//! - `picoserve::response::chunked::{Chunks, ChunkedResponse}` provides the
//!   streaming response body mechanism (chunked transfer encoding).
//! - There is no `{*wildcard}` path syntax; remaining-path routing is done
//!   with `Router::nest_service` + a `PathRouterService`, which receives the
//!   rest of the path (already split into URL-encoded segments).
//! - Path parameters are `parse_path_segment::<T: FromStr>()` inside a tuple
//!   path description (no `{name}` brace syntax in string routes).
//! - Request bodies are accessed via `request.body_connection.body()`; the
//!   `RequestBodyReader` obtained from `RequestBody::reader()` supports
//!   `with_different_timeout` to lift the RUDY-protection 3s read timeout for
//!   large uploads.

use core::fmt::Write as _;

use littlefs2::io::OpenSeekFrom;
use littlefs2::path::PathBuf;
use picoserve::io::Read;
use picoserve::response::chunked::{ChunkedResponse, ChunkWriter, Chunks, ChunksWritten};
use picoserve::request::{Path, Request};
use picoserve::response::{IntoResponse, ResponseWriter, StatusCode};
use picoserve::{ResponseSent, routing::PathRouterService};

use crate::apps::{self, AppParams};
use crate::storage::with_fs;

/// Size of a single LittleFS read/write chunk served or stored per lock hold.
pub const FS_CHUNK: usize = 512;

// ---------------------------------------------------------------------------
// MIME handling (C parity with `get_mime_type` in app_routing.c)
// ---------------------------------------------------------------------------

pub fn mime_for(path: &str) -> &'static str {
    let Some(dot) = path.rsplit_once('.') else {
        return "application/octet-stream";
    };
    let ext = dot.1;
    match ext {
        "html" | "htm" => "text/html",
        "js" | "mjs" => "application/javascript",
        "css" => "text/css",
        "svg" => "image/svg+xml",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

// ---------------------------------------------------------------------------
// Streaming file body from LittleFS
// ---------------------------------------------------------------------------

/// A [`Chunks`] impl that streams one LittleFS file in 512-byte chunks.
/// Each chunk re-opens the file (seek + read + close) so the FS mutex is only
/// held for microseconds at a time and SSE/web traffic is never starved.
pub struct AppFileChunks {
    path: PathBuf,
    mime: &'static str,
}

impl Chunks for AppFileChunks {
    fn content_type(&self) -> &'static str {
        self.mime
    }

    async fn write_chunks<W: picoserve::io::Write>(
        self,
        mut writer: ChunkWriter<W>,
    ) -> Result<ChunksWritten, W::Error> {
        let mut pos: u32 = 0;
        loop {
            // `read_chunk` opens, seeks, reads and closes the file, then
            // returns `(data, total_file_length)` — the `Option` layer is the
            // unmounted-filesystem case from `with_fs`.
            let read = with_fs(|fs| {
                fs.read_chunk::<FS_CHUNK>(&self.path, OpenSeekFrom::Start(pos))
                    .ok()
            });
            let Some((chunk, _file_len)) = read.flatten() else { break };
            if chunk.is_empty() {
                break;
            }
            writer.write_chunk(chunk.as_slice()).await?;
            pos += chunk.len() as u32;
        }
        writer.finalize().await
    }
}

/// Build the LittleFS path `/apps/{name}/{rest...}` from the URL-decoded
/// segments following the app name. Returns `None` on traversal attempts or
/// bad encoding.
fn app_fs_path(app: &AppParams, rest: &Path<'_>) -> Option<PathBuf> {
    let mut path = heapless::String::<255>::new();
    write!(path, "/apps/{}", app.as_str()).ok()?;
    let mut segments = rest.segments();
    let mut any = false;
    while let Some(seg) = segments.next() {
        let decoded = seg.try_into_string::<255>().ok()?;
        if decoded.is_empty() {
            continue; // collapses `a//b`
        }
        if decoded.contains('\\') || decoded.contains('\0') {
            return None;
        }
        for comp in decoded.split('/') {
            if comp.is_empty() || comp == "." {
                continue;
            }
            if comp == ".." {
                return None; // traversal guard
            }
            if !path.is_empty() && !path.ends_with('/') {
                path.push('/').ok()?;
            }
            path.push_str(comp).ok()?;
        }
        any = true;
    }
    if !any {
        return None; // bare `/apps/{name}` → nothing to serve
    }
    PathBuf::try_from(path.as_str()).ok()
}

/// `nest_service` handler for `/apps/{name}/{*path}`: serve files straight
/// from LittleFS, `{"status":"not_found"}` when missing.
pub struct AppFiles;

impl PathRouterService<(), (AppParams,)> for AppFiles {
    async fn call_path_router_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        (name,): (AppParams,),
        path: Path<'_>,
        request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if request.parts.method() != "GET" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let path_ = match app_fs_path(&name, &path) {
            Some(p) => p,
            None => {
                return not_found(request, response_writer).await;
            }
        };

        let is_file = with_fs(|fs| fs.metadata(&path_).map(|m| m.is_file()).unwrap_or(false))
            .unwrap_or(false);

        if !is_file {
            return not_found(request, response_writer).await;
        }

        let mime = mime_for(path_.as_str());
        let chunks = AppFileChunks { path: path_, mime };

        ChunkedResponse::new(chunks)
            .write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}

async fn not_found<R: Read, W: ResponseWriter<Error = R::Error>>(
    request: Request<'_, R>,
    response_writer: W,
) -> Result<ResponseSent, W::Error> {
    (
        StatusCode::NOT_FOUND,
        ("Content-Type", "application/json"),
        "{\"status\":\"not_found\"}",
    )
        .write_to(request.body_connection.finalize().await?, response_writer)
        .await
}

// ---------------------------------------------------------------------------
// Tarball install endpoint: POST /api/v1/apps/install/{name}
// ---------------------------------------------------------------------------

/// Custom [`picoserve::routing::MethodHandlerService`] so we can combine the
/// path parameter (app name) with access to the raw streaming request body —
/// handler-function extractors cannot see path parameters.
pub struct AppInstall;

impl picoserve::routing::MethodHandlerService<(), (AppParams,)> for AppInstall {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        (name,): (AppParams,),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "POST" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let report = apps::install_tarball(&name, request.body_connection.body()).await;

        // One concrete response type for all outcomes: a bounded JSON body in
        // a heapless string (no alloc, single match shape).
        let mut body = heapless::String::<128>::new();
        let mut code = StatusCode::OK;
        match report {
            Ok(r) => {
                let _ = write!(
                    body,
                    r#"{{"status":"installed","app":"{}","files":{},"bytes":{}}}"#,
                    r.app.as_str(),
                    r.files,
                    r.bytes
                );
            }
            Err(e) => {
                let _ = write!(
                    body,
                    "{}",
                    match e {
                        apps::InstallError::TooLarge => "{\"error\":\"payload_too_large\"}",
                        apps::InstallError::NotMounted => "{\"error\":\"fs_not_mounted\"}",
                        apps::InstallError::Incomplete => "{\"error\":\"incomplete_archive\"}",
                        apps::InstallError::Io | apps::InstallError::Failed(_) => {
                            "{\"error\":\"install_failed\"}"
                        }
                    }
                );
                code = match e {
                    apps::InstallError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
                    apps::InstallError::NotMounted => StatusCode::SERVICE_UNAVAILABLE,
                    apps::InstallError::Incomplete => StatusCode::BAD_REQUEST,
                    apps::InstallError::Io | apps::InstallError::Failed(_) => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                };
            }
        };

        (code, ("Content-Type", "application/json"), body.as_str())
            .write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}

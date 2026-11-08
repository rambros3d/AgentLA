//! Serving of uploaded app files out of LittleFS and the streaming tarball
//! installer endpoints, plus dynamic streaming of System WebUI assets from
//! SYSTEM_FS with transparent Gzip decompression / Content-Encoding: gzip.

use core::fmt::Write as _;

use littlefs2::io::OpenSeekFrom;
use littlefs2::path::PathBuf;
use picoserve::io::Read;
use picoserve::request::{Path, Request};
use picoserve::response::chunked::{ChunkWriter, ChunkedResponse, Chunks, ChunksWritten};
use picoserve::response::{IntoResponse, ResponseWriter, StatusCode};
use picoserve::routing::PathRouterService;
use picoserve::ResponseSent;

use crate::apps::{self, AppParams};
use crate::storage::{with_fs, with_system_fs};

/// Size of a single LittleFS read/write chunk served or stored per lock hold.
pub const FS_CHUNK: usize = 512;

// ---------------------------------------------------------------------------
// MIME handling
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
// Streaming file body from LittleFS (USER_FS)
// ---------------------------------------------------------------------------

/// A [`Chunks`] impl that streams one LittleFS file in 512-byte chunks.
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

/// Build the LittleFS path `/apps/{name}/{rest...}` from the URL-decoded segments.
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

/// `nest_service` handler for `/apps/{name}/{*path}`: serve files straight from LittleFS.
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
        if request.parts.method() != "GET" && request.parts.method() != "HEAD" {
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

        let is_user_file = with_fs(|fs| fs.metadata(&path_).map(|m| m.is_file()).unwrap_or(false))
            .unwrap_or(false);

        if is_user_file {
            let mime = mime_for(path_.as_str());
            let chunks = AppFileChunks { path: path_, mime };

            return ChunkedResponse::new(chunks)
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let is_sys_file = with_system_fs(|fs| fs.metadata(&path_).map(|m| m.is_file()).unwrap_or(false))
            .unwrap_or(false);

        if is_sys_file {
            let mime = mime_for(path_.as_str());
            let chunks = SystemFileChunks { path: path_, mime };

            return ChunkedResponse::new(chunks)
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        not_found(request, response_writer).await
    }
}

// ---------------------------------------------------------------------------
// Dynamic Streaming file body from SYSTEM_FS (WebUI partition)
// ---------------------------------------------------------------------------

/// A [`Chunks`] impl that streams one file from SYSTEM_FS in 512-byte chunks.
pub struct SystemFileChunks {
    path: PathBuf,
    mime: &'static str,
}

impl Chunks for SystemFileChunks {
    fn content_type(&self) -> &'static str {
        self.mime
    }

    async fn write_chunks<W: picoserve::io::Write>(
        self,
        mut writer: ChunkWriter<W>,
    ) -> Result<ChunksWritten, W::Error> {
        let mut pos: u32 = 0;
        loop {
            let read = with_system_fs(|fs| {
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

/// Request handler for dynamically serving System WebUI files out of Partition 1 (`SYSTEM_FS`).
/// Supports transparent pre-compressed gzip streaming with `Content-Encoding: gzip`
/// and falls back to uncompressed or emergency recovery page if partition is uninitialized.
pub struct SystemWebFile {
    file_name: &'static str,
    mime: &'static str,
}

impl SystemWebFile {
    pub const fn new(file_name: &'static str, mime: &'static str) -> Self {
        Self { file_name, mime }
    }
}

impl picoserve::routing::RequestHandlerService<(), ()> for SystemWebFile {
    async fn call_request_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if request.parts.method() != "GET" && request.parts.method() != "HEAD" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let client_accepts_gzip = request
            .parts
            .headers()
            .iter()
            .any(|(name, value)| {
                name == "accept-encoding"
                    && value.as_str().map(|v| v.contains("gzip")).unwrap_or(false)
            });

        // 1. If client accepts gzip, prioritize pre-compressed `.gz` asset from SYSTEM_FS
        let mut gz_name = heapless::String::<64>::new();
        let _ = write!(gz_name, "/{}.gz", self.file_name);
        let gz_path_opt = PathBuf::try_from(gz_name.as_str()).ok();

        let mut raw_name = heapless::String::<64>::new();
        let _ = write!(raw_name, "/{}", self.file_name);
        let raw_path_opt = PathBuf::try_from(raw_name.as_str()).ok();

        if client_accepts_gzip {
            if let Some(ref gz_path) = gz_path_opt {
                let exists = with_system_fs(|fs| {
                    fs.metadata(gz_path).map(|m| m.is_file()).unwrap_or(false)
                })
                .unwrap_or(false);

                if exists {
                    let chunks = SystemFileChunks {
                        path: gz_path.clone(),
                        mime: self.mime,
                    };
                    let resp = ChunkedResponse::new(chunks).into_response();
                    let resp = resp.with_header("Content-Encoding", "gzip");
                    let resp = if self.file_name.ends_with(".css") || self.file_name.ends_with(".js") {
                        resp.with_header("Cache-Control", "public, max-age=31536000")
                    } else {
                        resp.with_header("Cache-Control", "no-cache")
                    };
                    return resp
                        .write_to(request.body_connection.finalize().await?, response_writer)
                        .await;
                }
            }
        }

        // 2. Try raw uncompressed file from SYSTEM_FS
        if let Some(ref raw_path) = raw_path_opt {
            let exists = with_system_fs(|fs| {
                fs.metadata(raw_path).map(|m| m.is_file()).unwrap_or(false)
            })
            .unwrap_or(false);

            if exists {
                let chunks = SystemFileChunks {
                    path: raw_path.clone(),
                    mime: self.mime,
                };
                let resp = ChunkedResponse::new(chunks).into_response();
                let resp = if self.file_name.ends_with(".css") || self.file_name.ends_with(".js") {
                    resp.with_header("Cache-Control", "public, max-age=31536000")
                } else {
                    resp.with_header("Cache-Control", "no-cache")
                };
                return resp
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        }

        // 3. Fallback: serve .gz asset even if client omitted Accept-Encoding (HTTP/1.1 compatible)
        if let Some(ref gz_path) = gz_path_opt {
            let exists = with_system_fs(|fs| {
                fs.metadata(gz_path).map(|m| m.is_file()).unwrap_or(false)
            })
            .unwrap_or(false);

            if exists {
                let chunks = SystemFileChunks {
                    path: gz_path.clone(),
                    mime: self.mime,
                };
                let resp = ChunkedResponse::new(chunks).into_response();
                let resp = resp.with_header("Content-Encoding", "gzip");
                let resp = if self.file_name.ends_with(".css") || self.file_name.ends_with(".js") {
                    resp.with_header("Cache-Control", "public, max-age=31536000")
                } else {
                    resp.with_header("Cache-Control", "no-cache")
                };
                return resp
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        }

        // 4. Fallback recovery for HTML pages if partition is unpopulated
        if self.file_name == "index.html" || self.file_name == "apps.html" {
            const RECOVERY_HTML: &str = "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>AgentLA WebUI Setup</title><style>body{font-family:sans-serif;background:#181818;color:#eee;padding:2rem;line-height:1.5;}code{background:#333;padding:0.2rem 0.4rem;border-radius:4px;}</style></head><body><h2>AgentLA WebUI Partition Uninitialized</h2><p>The System WebUI filesystem partition is currently empty.</p><p>To initialize, please flash <code>webui.uf2</code> via USB BOOTSEL or upload a WebUI tarball to <code>POST /api/v1/system/webui/update</code>.</p></body></html>";
            return (
                StatusCode::OK,
                ("Content-Type", "text/html; charset=utf-8"),
                RECOVERY_HTML,
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        not_found(request, response_writer).await
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

// ---------------------------------------------------------------------------
// System WebUI Live Update endpoint: POST /api/v1/system/webui/update
// ---------------------------------------------------------------------------

pub struct SystemWebUiUpdate;

impl picoserve::routing::MethodHandlerService<()> for SystemWebUiUpdate {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
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

        let report = apps::update_system_webui(request.body_connection.body()).await;

        let mut body = heapless::String::<128>::new();
        let mut code = StatusCode::OK;
        match report {
            Ok(r) => {
                let _ = write!(
                    body,
                    r#"{{"status":"updated","files":{},"bytes":{}}}"#,
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
                            "{\"error\":\"update_failed\"}"
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

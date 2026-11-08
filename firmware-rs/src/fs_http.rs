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
pub const FS_CHUNK: usize = 2048;

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
        "md" | "markdown" => "text/markdown; charset=utf-8",
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
/// Defaults to `/apps/{name}/index.html` if no path or a trailing directory slash was supplied.
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
    if !any || path.ends_with('/') {
        if !path.ends_with('/') {
            path.push('/').ok()?;
        }
        path.push_str("index.html").ok()?;
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

        // If the path points to a directory in LittleFS, try index.html inside it
        let path_ = if with_fs(|fs| fs.metadata(&path_).map(|m| m.is_dir()).unwrap_or(false)).unwrap_or(false)
            || with_system_fs(|fs| fs.metadata(&path_).map(|m| m.is_dir()).unwrap_or(false)).unwrap_or(false)
        {
            let mut s = heapless::String::<255>::new();
            let _ = write!(s, "{}/index.html", path_.as_str().trim_end_matches('/'));
            PathBuf::try_from(s.as_str()).unwrap_or(path_)
        } else {
            path_
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
/// and redirects to /device if partition is unpopulated or corrupt.
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

        let is_html = self.file_name.ends_with(".html") || self.mime.contains("html");
        let cache_ctrl = if is_html {
            "no-cache, must-revalidate"
        } else {
            "public, max-age=604800, stale-while-revalidate=86400"
        };

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
                    let resp = resp.with_header("Cache-Control", cache_ctrl);
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
                let resp = resp.with_header("Cache-Control", cache_ctrl);
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
                let resp = resp.with_header("Cache-Control", cache_ctrl);
                return resp
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        }

        // 4. Fallback recovery for HTML pages if partition is unpopulated or corrupt -> redirect to /device
        if self.file_name == "index.html" || self.file_name == "lab.html" || self.file_name == "terminal.html" {
            return (
                StatusCode::FOUND,
                ("Location", "/device"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting to /device...",
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

/// Tarball install endpoint handling query parameter style: POST /api/v1/apps/install?name={name}
pub struct AppInstallQuery;

impl picoserve::routing::RequestHandlerService<(), ()> for AppInstallQuery {
    async fn call_request_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        let query = request.parts.query();
        let name_str = match crate::rest::query_val(query, "name") {
            Some(n) => n,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_name_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let name = match core::str::FromStr::from_str(name_str.as_str()) {
            Ok(n) => n,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_app_name\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

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
        }

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



// ---------------------------------------------------------------------------
// Filesystem REST API Endpoints: /api/v1/fs/*
// ---------------------------------------------------------------------------

pub fn decode_url_percent(input: &str) -> heapless::String<128> {
    let mut out = heapless::String::<128>::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex_slice = match core::str::from_utf8(&bytes[i + 1..i + 3]) {
                Ok(s) => s,
                Err(_) => {
                    let _ = out.push(bytes[i] as char);
                    i += 1;
                    continue;
                }
            };
            if let Ok(val) = u8::from_str_radix(hex_slice, 16) {
                let _ = out.push(val as char);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            let _ = out.push(' ');
            i += 1;
            continue;
        }
        let _ = out.push(bytes[i] as char);
        i += 1;
    }
    out
}

pub fn sanitize_fs_path(raw: &str) -> Option<heapless::String<128>> {
    let decoded = decode_url_percent(raw);
    let s = decoded.as_str().trim();
    if s.contains('\\') || s.contains('\0') {
        return None;
    }
    for seg in s.split('/') {
        if seg == ".." {
            return None;
        }
    }
    let mut clean = heapless::String::<128>::new();
    if !s.starts_with('/') {
        let _ = clean.push('/');
    }
    let mut last_was_slash = clean.ends_with('/');
    for ch in s.chars() {
        if ch == '/' {
            if !last_was_slash {
                let _ = clean.push('/');
                last_was_slash = true;
            }
        } else {
            let _ = clean.push(ch);
            last_was_slash = false;
        }
    }
    if clean.len() > 1 && clean.ends_with('/') {
        let _ = clean.pop();
    }
    if clean.is_empty() {
        let _ = clean.push('/');
    }
    Some(clean)
}

fn ensure_parent_dirs_user(
    fs: &littlefs2::fs::Filesystem<'static, crate::storage::UserFlashStorage>,
    full: &str,
) {
    let bytes = full.as_bytes();
    for i in 1..bytes.len() {
        if bytes[i] == b'/' {
            if let Ok(dir_path) = PathBuf::try_from(&full[..i]) {
                let _ = fs.create_dir_all(&dir_path);
            }
        }
    }
}

/// Helper to serialize directory entries into a JSON array buffer.
fn serialize_dir_entries<S: littlefs2::driver::Storage>(
    fs: &littlefs2::fs::Filesystem<'static, S>,
    path_str: &str,
    out: &mut heapless::String<2048>,
) -> Result<(), littlefs2::io::Error> {
    let mut first = true;
    let _ = out.push('[');

    let target_path = if path_str == "/" || path_str.is_empty() {
        PathBuf::try_from(".").map_err(|_| littlefs2::io::Error::IO)?
    } else {
        PathBuf::try_from(path_str).map_err(|_| littlefs2::io::Error::IO)?
    };

    let res = fs.read_dir_and_then(&target_path, |dir| {
        for entry in dir.by_ref() {
            let Ok(entry) = entry else { continue };
            let name = entry.file_name().as_str();
            if name == "." || name == ".." {
                continue;
            }
            if !first {
                let _ = out.push(',');
            }
            first = false;
            let m = entry.metadata();
            let is_dir = m.is_dir();
            let size = if is_dir { 0 } else { m.len() };
            let _ = write!(
                out,
                r#"{{"name":"{}","is_dir":{},"size":{}}}"#,
                name, is_dir, size
            );
        }
        Ok(())
    });

    if res.is_err() && (path_str == "/" || path_str.is_empty()) {
        if let Ok(root_path) = PathBuf::try_from("/") {
            out.clear();
            first = true;
            let _ = out.push('[');
            let _ = fs.read_dir_and_then(&root_path, |dir| {
                for entry in dir.by_ref() {
                    let Ok(entry) = entry else { continue };
                    let name = entry.file_name().as_str();
                    if name == "." || name == ".." {
                        continue;
                    }
                    if !first {
                        let _ = out.push(',');
                    }
                    first = false;
                    let m = entry.metadata();
                    let is_dir = m.is_dir();
                    let size = if is_dir { 0 } else { m.len() };
                    let _ = write!(
                        out,
                        r#"{{"name":"{}","is_dir":{},"size":{}}}"#,
                        name, is_dir, size
                    );
                }
                Ok(())
            });
        }
    }

    let _ = out.push(']');
    Ok(())
}

/// GET /api/v1/fs/stat
pub struct FsStat;

impl picoserve::routing::MethodHandlerService<()> for FsStat {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "GET" && method != "HEAD" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let (sys_used, sys_total) = crate::storage::system_fs_usage();
        let (usr_used, usr_total) = crate::storage::user_fs_usage();

        let mut body = heapless::String::<256>::new();
        let _ = write!(
            body,
            r#"{{"system":{{"total_bytes":{},"used_bytes":{},"read_only":true}},"user":{{"total_bytes":{},"used_bytes":{},"read_only":false}}}}"#,
            sys_total, sys_used, usr_total, usr_used
        );

        let finalizer = request.body_connection.finalize().await?;
        (
            StatusCode::OK,
            [
                ("Content-Type", "application/json"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
            ],
            body.as_str(),
        )
            .write_to(finalizer, response_writer)
            .await
    }
}

/// GET /api/v1/fs/list?root={system|user}&path={path}
pub struct FsList;

impl picoserve::routing::MethodHandlerService<()> for FsList {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "GET" && method != "HEAD" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());
        let raw_path = crate::rest::query_val(query, "path")
            .unwrap_or_else(|| heapless::String::try_from("/").unwrap());

        let clean_path = match sanitize_fs_path(raw_path.as_str()) {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let mut out = heapless::String::<2048>::new();
        let is_system = root.as_str() == "system";

        let outcome = if is_system {
            with_system_fs(|fs| serialize_dir_entries(fs, clean_path.as_str(), &mut out))
        } else {
            crate::storage::with_user_fs(|fs| serialize_dir_entries(fs, clean_path.as_str(), &mut out))
        };

        let (code, body): (StatusCode, &str) = match outcome {
            Some(Ok(())) => (StatusCode::OK, out.as_str()),
            Some(Err(_)) => (StatusCode::NOT_FOUND, "{\"error\":\"directory_not_found\"}"),
            None => (StatusCode::SERVICE_UNAVAILABLE, "{\"error\":\"fs_not_mounted\"}"),
        };

        (
            code,
            [
                ("Content-Type", "application/json"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
            ],
            body,
        )
            .write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}

/// GET /api/v1/fs/read?root={system|user}&path={path}
pub struct FsRead;

impl picoserve::routing::MethodHandlerService<()> for FsRead {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "GET" && method != "HEAD" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());
        let raw_path = match crate::rest::query_val(query, "path") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_path_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_path = match sanitize_fs_path(raw_path.as_str()) {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let pbuf = match PathBuf::try_from(clean_path.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"path_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let is_system = root.as_str() == "system";
        let mime = mime_for(clean_path.as_str());

        if is_system {
            let exists = with_system_fs(|fs| fs.metadata(&pbuf).map(|m| m.is_file()).unwrap_or(false))
                .unwrap_or(false);
            if !exists {
                return (
                    StatusCode::NOT_FOUND,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"file_not_found\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }

            let chunks = SystemFileChunks { path: pbuf, mime };
            let resp = ChunkedResponse::new(chunks).into_response();
            let resp = resp.with_header("Cache-Control", "no-cache, no-store, must-revalidate");
            return resp
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let exists = crate::storage::with_user_fs(|fs| fs.metadata(&pbuf).map(|m| m.is_file()).unwrap_or(false))
            .unwrap_or(false);
        if !exists {
            return (
                StatusCode::NOT_FOUND,
                ("Content-Type", "application/json"),
                "{\"error\":\"file_not_found\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let chunks = AppFileChunks { path: pbuf, mime };
        let resp = ChunkedResponse::new(chunks).into_response();
        let resp = resp.with_header("Cache-Control", "no-cache, no-store, must-revalidate");
        resp.write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}

/// POST /api/v1/fs/write?root={root}&path={path}
/// Body contains raw data to write.
/// If root=system, strictly rejects with 403 Forbidden.
pub struct FsWrite;

impl picoserve::routing::MethodHandlerService<()> for FsWrite {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "POST" && method != "PUT" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());

        // Strict partition boundary enforcement: system_fs is immutable ROM
        if root.as_str() == "system" {
            return (
                StatusCode::FORBIDDEN,
                ("Content-Type", "application/json"),
                "{\"error\":\"system_fs is read-only\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let raw_path = match crate::rest::query_val(query, "path") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_path_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_path = match sanitize_fs_path(raw_path.as_str()) {
            Some(p) if p.as_str() != "/" => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let pbuf = match PathBuf::try_from(clean_path.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"path_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        // Truncate/create file and ensure parent dirs exist
        let init_res = crate::storage::with_user_fs(|fs| {
            ensure_parent_dirs_user(fs, clean_path.as_str());
            fs.write(&pbuf, &[])
        });

        match init_res {
            Some(Ok(())) => {}
            Some(Err(_)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"fs_init_write_failed\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"fs_not_mounted\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        }

        // Stream body chunks into LittleFS
        let mut reader = request.body_connection.body().reader();
        let mut buf = [0u8; 512];
        let mut pos: u32 = 0;
        let mut write_err = false;

        loop {
            let n = match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => {
                    write_err = true;
                    break;
                }
            };

            let res = crate::storage::with_user_fs(|fs| {
                fs.write_chunk(&pbuf, &buf[..n], OpenSeekFrom::Start(pos))
            });

            match res {
                Some(Ok(())) => {
                    pos += n as u32;
                }
                _ => {
                    write_err = true;
                    break;
                }
            }
        }

        let finalizer = request.body_connection.finalize().await?;
        if write_err {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ("Content-Type", "application/json"),
                "{\"error\":\"fs_write_chunk_failed\"}",
            )
                .write_to(finalizer, response_writer)
                .await
        } else {
            let mut resp = heapless::String::<128>::new();
            let _ = write!(
                resp,
                r#"{{"status":"ok","bytes_written":{},"path":"{}"}}"#,
                pos,
                clean_path.as_str()
            );
            (
                StatusCode::OK,
                [
                    ("Content-Type", "application/json"),
                    ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ],
                resp.as_str(),
            )
                .write_to(finalizer, response_writer)
                .await
        }
    }
}

/// DELETE /api/v1/fs/entry?root={root}&path={path}
/// Also supports POST /api/v1/fs/entry?root={root}&path={path}
/// Strictly rejects with 403 Forbidden if root=system.
pub struct FsEntry;

impl picoserve::routing::MethodHandlerService<()> for FsEntry {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "DELETE" && method != "POST" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());

        if root.as_str() == "system" {
            return (
                StatusCode::FORBIDDEN,
                ("Content-Type", "application/json"),
                "{\"error\":\"system_fs is read-only\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let raw_path = match crate::rest::query_val(query, "path") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_path_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_path = match sanitize_fs_path(raw_path.as_str()) {
            Some(p) if p.as_str() != "/" => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"cannot_delete_root\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let pbuf = match PathBuf::try_from(clean_path.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"path_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let outcome = crate::storage::with_user_fs(|fs| {
            let is_dir = fs.metadata(&pbuf).map(|m| m.is_dir()).unwrap_or(false);
            if is_dir {
                fs.remove_dir_all(&pbuf)
            } else {
                fs.remove(&pbuf)
            }
        });

        let finalizer = request.body_connection.finalize().await?;
        match outcome {
            Some(Ok(())) => {
                let mut resp = heapless::String::<128>::new();
                let _ = write!(resp, r#"{{"status":"deleted","path":"{}"}}"#, clean_path.as_str());
                (StatusCode::OK, ("Content-Type", "application/json"), resp.as_str())
                    .write_to(finalizer, response_writer)
                    .await
            }
            Some(Err(_)) => {
                (StatusCode::NOT_FOUND, ("Content-Type", "application/json"), "{\"error\":\"entry_not_found\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
            None => {
                (StatusCode::SERVICE_UNAVAILABLE, ("Content-Type", "application/json"), "{\"error\":\"fs_not_mounted\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
        }
    }
}

/// POST /api/v1/fs/mkdir?root={root}&path={path}
/// Strictly rejects with 403 Forbidden if root=system.
pub struct FsMkdir;

impl picoserve::routing::MethodHandlerService<()> for FsMkdir {
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

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());

        if root.as_str() == "system" {
            return (
                StatusCode::FORBIDDEN,
                ("Content-Type", "application/json"),
                "{\"error\":\"system_fs is read-only\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let raw_path = match crate::rest::query_val(query, "path") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_path_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_path = match sanitize_fs_path(raw_path.as_str()) {
            Some(p) if p.as_str() != "/" => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let pbuf = match PathBuf::try_from(clean_path.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"path_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let outcome = crate::storage::with_user_fs(|fs| fs.create_dir_all(&pbuf));
        let finalizer = request.body_connection.finalize().await?;

        match outcome {
            Some(Ok(())) => {
                let mut resp = heapless::String::<128>::new();
                let _ = write!(resp, r#"{{"status":"created","path":"{}"}}"#, clean_path.as_str());
                (StatusCode::OK, ("Content-Type", "application/json"), resp.as_str())
                    .write_to(finalizer, response_writer)
                    .await
            }
            Some(Err(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, ("Content-Type", "application/json"), "{\"error\":\"mkdir_failed\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
            None => {
                (StatusCode::SERVICE_UNAVAILABLE, ("Content-Type", "application/json"), "{\"error\":\"fs_not_mounted\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
        }
    }
}

/// POST /api/v1/fs/rename?root={root}&src={src}&dst={dst}
/// Strictly rejects with 403 Forbidden if root=system.
pub struct FsRename;

impl picoserve::routing::MethodHandlerService<()> for FsRename {
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

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());

        if root.as_str() == "system" {
            return (
                StatusCode::FORBIDDEN,
                ("Content-Type", "application/json"),
                "{\"error\":\"system_fs is read-only\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let raw_src = match crate::rest::query_val(query, "src") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_src_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let raw_dst = match crate::rest::query_val(query, "dst") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_dst_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_src = match sanitize_fs_path(raw_src.as_str()) {
            Some(p) if p.as_str() != "/" => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_src_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_dst = match sanitize_fs_path(raw_dst.as_str()) {
            Some(p) if p.as_str() != "/" => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_dst_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let src_pbuf = match PathBuf::try_from(clean_src.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"src_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let dst_pbuf = match PathBuf::try_from(clean_dst.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"dst_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let outcome = crate::storage::with_user_fs(|fs| {
            ensure_parent_dirs_user(fs, clean_dst.as_str());
            fs.rename(&src_pbuf, &dst_pbuf)
        });

        let finalizer = request.body_connection.finalize().await?;
        match outcome {
            Some(Ok(())) => {
                let mut resp = heapless::String::<256>::new();
                let _ = write!(
                    resp,
                    r#"{{"status":"renamed","src":"{}","dst":"{}"}}"#,
                    clean_src.as_str(),
                    clean_dst.as_str()
                );
                (StatusCode::OK, ("Content-Type", "application/json"), resp.as_str())
                    .write_to(finalizer, response_writer)
                    .await
            }
            Some(Err(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, ("Content-Type", "application/json"), "{\"error\":\"rename_failed\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
            None => {
                (StatusCode::SERVICE_UNAVAILABLE, ("Content-Type", "application/json"), "{\"error\":\"fs_not_mounted\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
        }
    }
}

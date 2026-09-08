//! Axum owns path and method routing. Each handler calls a file operation;
//! there is no fallback into a second complete HTTP dispatcher.
use super::{
    MutationProgress,
    files::{FileAction, FileRequest},
    request::RequestProfile,
};
use crate::{
    auth::FilePrincipal,
    server::{
        Request, Response, Server,
        operation_registry::{
            JOB_STATUS_PREFIX, apply_invalid_job_id, apply_status, parse_canonical_operation_id,
        },
        path_policy::RoutePath,
        problem::{ApiError, ErrorCode, render_problem},
        status_method_not_allowed, status_not_found,
    },
};
use axum::{
    Router,
    extract::State,
    middleware::{self, Next},
    routing::{MethodFilter, any, on},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub(super) struct OmitHeadContentLength;

#[derive(Clone, Default)]
pub(super) struct AuthLog(pub Arc<Mutex<Option<String>>>);

#[derive(Clone)]
pub(super) struct BusinessError(pub Arc<Mutex<Option<anyhow::Error>>>);

pub(super) fn assemble(
    server: Arc<Server>,
    handle: sarmg_server_runtime::RuntimeHandle,
) -> anyhow::Result<Router> {
    let file_methods = on(MethodFilter::GET, file_get)
        .on(MethodFilter::HEAD, file_head)
        .on(MethodFilter::PUT, file_put)
        .on(MethodFilter::PATCH, file_patch)
        .on(MethodFilter::DELETE, file_delete)
        .fallback(file_method_denied);
    let protected = Router::new()
        .route("/", file_methods.clone())
        .route("/{*path}", file_methods)
        .route(
            "/__dufs__/api/list",
            on(MethodFilter::GET, list)
                .on(MethodFilter::HEAD, get_api_method_denied)
                .fallback(get_api_method_denied),
        )
        .route(
            "/__dufs__/api/jobs/{id}",
            on(MethodFilter::GET, job)
                .on(MethodFilter::HEAD, get_api_method_denied)
                .fallback(get_api_method_denied),
        )
        .route(
            "/__dufs__/api/mkdir",
            on(MethodFilter::POST, mkdir).fallback(post_api_method_denied),
        )
        .route(
            "/__dufs__/api/move",
            on(MethodFilter::POST, move_entry).fallback(post_api_method_denied),
        )
        .route(
            "/__dufs__/api/rename",
            on(MethodFilter::POST, rename).fallback(post_api_method_denied),
        )
        .route(
            "/__dufs__/api/upload/preflight",
            on(MethodFilter::POST, preflight).fallback(post_api_method_denied),
        )
        .route(
            "/__dufs__/api/upload/discard",
            on(MethodFilter::POST, discard).fallback(post_api_method_denied),
        )
        .route("/__dufs__", any(unknown_internal))
        .route("/__dufs__/{*path}", any(unknown_internal))
        .route_layer(middleware::from_fn_with_state(server.clone(), authenticate))
        .with_state(server.clone());
    let public = Router::new()
        .route(
            "/__dufs__/login",
            on(MethodFilter::GET, login)
                .on(MethodFilter::HEAD, login_method_denied)
                .fallback(login_method_denied),
        )
        .route(
            &format!("/{}{{*asset}}", server.content.assets_prefix),
            on(MethodFilter::GET, asset)
                .on(MethodFilter::HEAD, asset)
                .fallback(asset_method_denied),
        )
        .route("/api/v2/auth", any(unknown_platform))
        .route("/api/v2/auth/{*path}", any(unknown_platform))
        .with_state(server.clone());
    let platform = sarmg_server_runtime::platform_router(
        handle,
        "dufs-ram",
        server.content.administrator_origin,
        server.content.administrator.clone(),
    )?;
    Ok(protected.merge(public).merge(platform))
}

async fn authenticate(
    State(server): State<Arc<Server>>,
    mut request: Request,
    next: Next,
) -> Response {
    match sarmg_admin_axum::authenticate_request(
        &server.content.administrator,
        request.headers(),
        request.uri(),
        request.method(),
        "dufs-ram",
        server.content.administrator_origin,
    )
    .await
    {
        Ok(identity) => {
            if let Some(log) = request.extensions().get::<AuthLog>() {
                *log.0.lock().expect("auth log lock") = Some(identity.username.clone());
            }
            request.extensions_mut().insert(FilePrincipal {
                // Account renames must not orphan uploads or transfer ownership.
                username: identity.administrator_id.to_string(),
            });
            next.run(request).await
        }
        Err(response) => {
            let mut response = *response;
            let api = request
                .extensions()
                .get::<RequestProfile>()
                .is_some_and(RequestProfile::is_internal_api);
            server.redirect_unauthenticated_page(
                request.method(),
                request.headers(),
                api,
                &mut response,
            );
            response
        }
    }
}

fn finish(result: anyhow::Result<Response>) -> Response {
    match result {
        Ok(response) => response,
        Err(error) => {
            let mut response = Response::default();
            response
                .extensions_mut()
                .insert(BusinessError(Arc::new(Mutex::new(Some(error)))));
            response
        }
    }
}
fn principal(request: &Request) -> FilePrincipal {
    request
        .extensions()
        .get::<FilePrincipal>()
        .expect("protected router authenticated the request")
        .clone()
}
fn mutation(request: &Request) -> MutationProgress {
    request
        .extensions()
        .get::<MutationProgress>()
        .expect("request boundary installed mutation context")
        .clone()
}
fn route_path(request: &Request) -> RoutePath {
    request
        .extensions()
        .get::<RoutePath>()
        .expect("pre-router boundary validated the raw path")
        .clone()
}

async fn file(server: Arc<Server>, request: Request, action: FileAction) -> Response {
    let principal = principal(&request);
    let path = route_path(&request);
    let progress = mutation(&request);
    let head = request.method() == http::Method::HEAD;
    let mut response = finish(
        FileRequest::new(server, request, path, progress)
            .execute(action, &principal)
            .await,
    );
    // Axum infers Content-Length: 0 from an empty HEAD body. Directory HEAD
    // and upload-status HEAD deliberately make no representation-length claim.
    if head
        && !response
            .headers()
            .contains_key(http::header::CONTENT_LENGTH)
    {
        response.extensions_mut().insert(OmitHeadContentLength);
    }
    response
}

async fn file_get(State(server): State<Arc<Server>>, request: Request) -> Response {
    file(server, request, FileAction::Read).await
}
async fn file_head(State(server): State<Arc<Server>>, request: Request) -> Response {
    file(server, request, FileAction::Read).await
}
async fn file_put(State(server): State<Arc<Server>>, request: Request) -> Response {
    file(server, request, FileAction::Upload).await
}
async fn file_patch(State(server): State<Arc<Server>>, request: Request) -> Response {
    file(server, request, FileAction::Resume).await
}
async fn file_delete(State(server): State<Arc<Server>>, request: Request) -> Response {
    file(server, request, FileAction::Delete).await
}

async fn login(State(server): State<Arc<Server>>) -> Response {
    let mut response = Response::default();
    finish(
        server
            .send_login_page_for_get(&mut response)
            .map(|()| response),
    )
}
async fn asset(State(server): State<Arc<Server>>, request: Request) -> Response {
    let mut response = Response::default();
    if !server.handle_internal(
        route_path(&request).as_str(),
        request.method() == http::Method::HEAD,
        &mut response,
    ) {
        status_not_found(&mut response);
    }
    response
}
async fn list(State(server): State<Arc<Server>>, request: Request) -> Response {
    let parameters: HashMap<_, _> =
        form_urlencoded::parse(request.uri().query().unwrap_or_default().as_bytes())
            .into_owned()
            .collect();
    let mut response = Response::default();
    finish(
        server
            .handle_list_api(&principal(&request).username, &parameters, &mut response)
            .await
            .map(|()| response),
    )
}
async fn job(State(server): State<Arc<Server>>, request: Request) -> Response {
    let path = route_path(&request);
    let owner = principal(&request).username;
    let id = path
        .as_str()
        .strip_prefix(JOB_STATUS_PREFIX)
        .and_then(parse_canonical_operation_id);
    let mut response = Response::default();
    let Some(id) = id else {
        return finish(
            apply_invalid_job_id(
                &mut response,
                "Job status path must end in a canonical UUID",
            )
            .map(|()| response),
        );
    };
    finish(
        async move {
            let status = server.state.operation_registry.status(&owner, id).await?;
            apply_status(&mut response, id, status)?;
            Ok(response)
        }
        .await,
    )
}
async fn browser_mutation(
    server: Arc<Server>,
    request: Request,
    endpoint: &'static str,
) -> Response {
    let owner = principal(&request).username;
    let mutation = mutation(&request);
    let mut response = Response::default();
    finish(
        server
            .handle_browser_api(endpoint, &owner, request, mutation, &mut response)
            .await
            .map(|()| response),
    )
}
async fn mkdir(State(server): State<Arc<Server>>, request: Request) -> Response {
    browser_mutation(server, request, "__dufs__/api/mkdir").await
}
async fn move_entry(State(server): State<Arc<Server>>, request: Request) -> Response {
    browser_mutation(server, request, "__dufs__/api/move").await
}
async fn rename(State(server): State<Arc<Server>>, request: Request) -> Response {
    browser_mutation(server, request, "__dufs__/api/rename").await
}
async fn preflight(State(server): State<Arc<Server>>, request: Request) -> Response {
    let owner = principal(&request).username;
    let mut response = Response::default();
    finish(
        server
            .handle_upload_preflight(&owner, request, &mut response)
            .await
            .map(|()| response),
    )
}
async fn discard(State(server): State<Arc<Server>>, request: Request) -> Response {
    let owner = principal(&request).username;
    let mut response = Response::default();
    finish(
        server
            .handle_upload_discard(&owner, request, &mut response)
            .await
            .map(|()| response),
    )
}
fn method_denied(allow: &'static str, api: bool) -> Response {
    let mut response = Response::default();
    status_method_not_allowed(&mut response, allow);
    if api {
        return finish(
            render_problem(
                &mut response,
                &ApiError::new(
                    http::StatusCode::METHOD_NOT_ALLOWED,
                    ErrorCode::METHOD_NOT_ALLOWED,
                    "Method not allowed for this API endpoint",
                ),
            )
            .map(|()| response),
        );
    }
    response
}
async fn get_api_method_denied() -> Response {
    method_denied("GET", true)
}
async fn post_api_method_denied() -> Response {
    method_denied("POST", true)
}
async fn login_method_denied() -> Response {
    method_denied("GET", false)
}
async fn asset_method_denied() -> Response {
    method_denied("GET, HEAD", false)
}
async fn file_method_denied() -> Response {
    method_denied("GET, HEAD, PUT, PATCH, DELETE", false)
}
async fn unknown_internal() -> Response {
    let mut response = Response::default();
    finish(
        render_problem(
            &mut response,
            &ApiError::new(
                http::StatusCode::NOT_FOUND,
                ErrorCode::API_ENDPOINT_NOT_FOUND,
                "API endpoint not found",
            ),
        )
        .map(|()| response),
    )
}
async fn unknown_platform() -> Response {
    let mut response = Response::default();
    status_not_found(&mut response);
    response
}

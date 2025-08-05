use actix_web::{dev::{Service, ServiceRequest, ServiceResponse, Transform}, error, Error};
use futures::future::{ready, LocalBoxFuture, Ready};

/// Authentication middleware (认证中间件)
#[derive(Clone)]
pub struct AuthMiddleware {
    auth_token: String,
}

impl AuthMiddleware {
    /// Create new authentication middleware instance (创建新的认证中间件实例)
    pub fn new(auth_token: String) -> Self {
        Self { auth_token }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service,
            auth_token: self.auth_token.clone(),
        }))
    }
}

/// Authentication middleware service implementation (认证中间件服务实现)
pub struct AuthMiddlewareService<S> {
    service: S,
    auth_token: String,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // 如果是下载路径或统计页面，跳过认证
        if req.path().starts_with("/avatars/") || req.path() == "/stats" {
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await });
        }

        // 检查Authorization头
        let auth_header = match req.headers().get("Authorization") {
            Some(header) => header.to_str().unwrap_or(""),
            None => {
                return Box::pin(async move {
                    Err(error::ErrorUnauthorized("No authorization header"))
                });
            }
        };

        if !auth_header.starts_with("Bearer ") {
            return Box::pin(async move {
                Err(error::ErrorUnauthorized("Invalid authorization header format"))
            });
        }

        let token = &auth_header[7..];
        if token != self.auth_token {
            return Box::pin(async move { Err(error::ErrorUnauthorized("Invalid token")) });
        }

        // 通过认证，继续处理请求
        let fut = self.service.call(req);
        Box::pin(async move { fut.await })
    }
}
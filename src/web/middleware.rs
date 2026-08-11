use actix_web::{
    Error, HttpResponse,
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    web,
};
use redb::Database;
use std::{
    future::{Future, Ready, ready},
    pin::Pin,
};

pub struct RequireAuth;

impl<S, B> Transform<S, ServiceRequest> for RequireAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RequireAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequireAuthMiddleware { service }))
    }
}

pub struct RequireAuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequireAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Auth endpoints are always public
        if req.path().starts_with("/api/auth/") {
            let fut = self.service.call(req);
            return Box::pin(async move { Ok(fut.await?.map_into_left_body()) });
        }

        let db = match req.app_data::<web::Data<Database>>() {
            Some(db) => db.clone(),
            None => {
                log::error!("RequireAuth: Database not found in app_data — this is a bug");
                let (req, _) = req.into_parts();
                return Box::pin(async move {
                    let res = HttpResponse::InternalServerError()
                        .json(serde_json::json!({"error": "internal error"}));
                    Ok(ServiceResponse::new(req, res).map_into_right_body())
                });
            }
        };

        let authenticated = req
            .cookie(crate::session::COOKIE_NAME)
            .and_then(|c| crate::db::get_session(&db, c.value()))
            .is_some();

        if !authenticated {
            let (req, _) = req.into_parts();
            return Box::pin(async move {
                let res =
                    HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}));
                Ok(ServiceResponse::new(req, res).map_into_right_body())
            });
        }

        let fut = self.service.call(req);
        Box::pin(async move { Ok(fut.await?.map_into_left_body()) })
    }
}

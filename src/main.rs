use actix_web::{
    delete, get, post,
    web::{self, JsonConfig},
    App, HttpResponse, HttpServer, Responder,
};
use derivadex::{Account, Engine, EngineError, Order};
use displaydoc::Display;
use std::{sync::Mutex, time::SystemTime};
use thiserror::Error;
use web3::types::{Address, H256};

#[derive(Debug, Display, Error)]
enum DerivadexError {
    /// engine error: {0}
    EngineError(#[from] EngineError),
}

impl actix_web::error::ResponseError for DerivadexError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            DerivadexError::EngineError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[post("/")]
async fn create_account(
    engine: web::Data<Mutex<Engine>>,
    request: web::Json<Account>,
) -> impl Responder {
    let account = request.into_inner();
    let address = engine.lock().unwrap().create_account(account)?;
    Ok::<HttpResponse, DerivadexError>(HttpResponse::Ok().body(format!("{:#x}", address)))
}

#[get("/{traderAddress}")]
async fn get_account(
    engine: web::Data<Mutex<Engine>>,
    trader_address: web::Path<Address>,
) -> impl Responder {
    let account = engine.lock().unwrap().get_account(*trader_address)?;
    Ok::<HttpResponse, DerivadexError>(HttpResponse::Ok().json(account))
}

#[delete("/{traderAddress}")]
async fn delete_account(
    engine: web::Data<Mutex<Engine>>,
    trader_address: web::Path<Address>,
) -> impl Responder {
    engine.lock().unwrap().delete_account(*trader_address)?;
    Ok::<HttpResponse, DerivadexError>(HttpResponse::Ok().finish())
}

#[post("/")]
async fn create_order(
    engine: web::Data<Mutex<Engine>>,
    mut request: web::Json<Order>,
) -> impl Responder {
    request.timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fills = engine.lock().unwrap().create_order(*request)?;
    Ok::<HttpResponse, DerivadexError>(HttpResponse::Ok().json(fills))
}

#[get("/{hash}")]
async fn get_order(
    engine: web::Data<Mutex<Engine>>,
    order_hash: web::Path<H256>,
) -> impl Responder {
    let order = engine.lock().unwrap().get_order(*order_hash)?;
    Ok::<HttpResponse, DerivadexError>(HttpResponse::Ok().json(order))
}

#[delete("/{hash}")]
async fn delete_order(
    engine: web::Data<Mutex<Engine>>,
    order_hash: web::Path<H256>,
) -> impl Responder {
    engine.lock().unwrap().delete_order(*order_hash)?;
    Ok::<HttpResponse, DerivadexError>(HttpResponse::Ok().finish())
}

#[get("/book")]
async fn get_book(engine: web::Data<Mutex<Engine>>) -> impl Responder {
    let l2_order_book = engine.lock().unwrap().get_book();
    Ok::<HttpResponse, DerivadexError>(HttpResponse::Ok().json(l2_order_book))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_data = web::Data::new(Mutex::new(Engine::new()));
    HttpServer::new(move || {
        App::new()
            .app_data(JsonConfig::default().error_handler(|err, _| {
                actix_web::error::InternalError::from_response(
                    err,
                    HttpResponse::BadRequest().into(),
                )
                .into()
            }))
            .app_data(app_data.clone())
            .service(
                web::scope("/accounts")
                    .service(create_account)
                    .service(get_account)
                    .service(delete_account),
            )
            .service(
                web::scope("/orders")
                    .service(create_order)
                    .service(get_order)
                    .service(delete_order),
            )
            .service(get_book)
    })
    .bind(("127.0.0.1", 4321))?
    .run()
    .await
}

use hyper::server::Request;
use hyper::status::StatusCode;
use hyper::method::Method;
use uuid::Uuid;

use super::super::{Result, ErrorKind};
use super::super::sdk::SdkInfo;
use super::server::{ServerContext, load_request_data};
use super::types::{ApiResponse, ApiError};

#[derive(Deserialize)]
struct SymbolQuery {
    addr: u64,
    image_addr: u64,
    image_vmaddr: Option<u64>,
    image_uuid: Option<Uuid>,
    image_path: Option<String>,
}

#[derive(Deserialize)]
struct SymbolLookupRequest {
    sdk_id: String,
    cpu_name: String,
    symbols: Vec<SymbolQuery>,
}

#[derive(Serialize)]
struct Symbol {
    object_name: String,
    symbol: String,
    addr: u64,
}

#[derive(Serialize)]
struct SymbolResponse {
    symbols: Vec<Option<Symbol>>,
}

pub fn healthcheck_handler(ctx: &ServerContext, req: Request) -> Result<ApiResponse>
{
    if req.method != Method::Get {
        return Err(ApiError::MethodNotAllowed.into());
    }
    let rv = ctx.check_health()?;
    let status = if rv.is_healthy {
        StatusCode::Ok
    } else {
        StatusCode::ServiceUnavailable
    };
    ApiResponse::new(rv, status)
}

pub fn lookup_symbol_handler(ctx: &ServerContext, mut req: Request) -> Result<ApiResponse>
{
    if req.method != Method::Post {
        return Err(ApiError::MethodNotAllowed.into());
    }
    let data : SymbolLookupRequest = load_request_data(&mut req)?;

    let sdk = match ctx.stash.get_memdb_from_sdk_id(&data.sdk_id) {
        Ok(sdk) => sdk,
        Err(err) => {
            if let &ErrorKind::UnknownSdk = err.kind() {
                return Err(ApiError::SdkNotFound.into());
            } else {
                return Err(err)
            }
        },
    };

    let mut rv = vec![];
    for symbol_query in data.symbols {
        let mut rvsym = None;
        if let Some(ref uuid) = symbol_query.image_uuid {
            if let Some(sym) = sdk.lookup_by_uuid(uuid, symbol_query.addr) {
                rvsym = Some(sym);
            }
        } else if let Some(ref name) = symbol_query.image_path {
            if let Some(sym) = sdk.lookup_by_object_name(
                name, &data.cpu_name, symbol_query.addr)
            {
                rvsym = Some(sym);
            }
        }
        rv.push(rvsym.map(|sym| {
            Symbol {
                object_name: sym.object_name().to_string(),
                symbol: sym.symbol().to_string(),
                addr: sym.addr(),
            }
        }));
    }

    ApiResponse::new(SymbolResponse {
        symbols: rv,
    }, StatusCode::Ok)
}

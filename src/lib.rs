use serde::{Deserialize, Serialize};
use serde_json::json;
use worker::*;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}
#[derive(Deserialize, Serialize)]
struct RequestBody {
    name: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct UserRequest {
    name: String,
    email: String,
    password: String,
}

#[derive(Deserialize, Serialize)]
struct UserResponse {
    name: String,
    email: String,
}

#[derive(Deserialize, Serialize)]
struct UserStored {
    name: String,
    email: String,
    password: String,
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get("/", |_, _| Response::ok("Hello from Workers!"))
        .post_async("/users", |mut req, ctx| async move {
            let body = req.json::<UserRequest>().await;
            let store = ctx.env.kv("users")?;
            match body {
                Ok(user) => {
                    let created_user = UserResponse {
                        name: String::from(&user.name),
                        email: String::from(&user.email),
                    };
                    let user_to_save = UserStored {
                        name: String::from(&user.name),
                        email: String::from(&user.email),
                        password: user.password,
                    };

                    let db_result = store.put(&user.email, user_to_save)?.execute().await?;
                    Response::from_json(&created_user)
                }
                Err(err) => return Response::error("Wrong Data", 400),
            }
        })
        .get_async("/users/:id", |_, ctx| async move {
            if let Some(id) = ctx.param("id") {
                let users = ctx.env.kv("users")?;
                let user = users.get(id).json::<UserStored>().await?;
                match user {
                    Some(found) => return Response::from_json(&found),
                    None => return Response::error("User not found", 404),
                };
            }
            Response::error("Bad Request", 400)
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}

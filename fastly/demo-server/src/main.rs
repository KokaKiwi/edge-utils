use fastly::{Error, KVStore, Request, Response};

#[fastly::main]
fn main(req: Request) -> Result<Response, Error> {
    // Open the KV Store named "data"
    let store = KVStore::open("data")?.expect("KV Store 'data' not found");

    // Get the request path and remove the leading '/'
    let key = req.get_path().trim_start_matches('/');

    // Retrieve the value for the key from the request path
    let mut value = store.lookup(key)?;
    let content = value.take_body_bytes();

    Ok(Response::from_body(content))
}

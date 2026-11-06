use std::slice;
use std::sync::Mutex;

use rquickjs::{Context, Function, Runtime, Value};

const HOST_BUFFER_BYTES: usize = 256 * 1024;

static RESULT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static mut HOST_BUFFER: [u8; HOST_BUFFER_BYTES] = [0; HOST_BUFFER_BYTES];

#[link(wasm_import_module = "env")]
extern "C" {
    fn host_tool_call(req_ptr: u32, req_len: u32) -> u32;
    fn host_log(ptr: u32, len: u32);
}

#[no_mangle]
pub extern "C" fn alloc(len: u32) -> u32 {
    let mut buffer = Vec::<u8>::with_capacity(len as usize);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr as u32
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: u32, len: u32) {
    unsafe {
        drop(Vec::from_raw_parts(ptr as *mut u8, len as usize, len as usize));
    }
}

#[no_mangle]
pub extern "C" fn host_buffer() -> u32 {
    std::ptr::addr_of_mut!(HOST_BUFFER) as u32
}

#[no_mangle]
pub extern "C" fn eval(script_ptr: u32, script_len: u32) -> i32 {
    let script = unsafe { slice::from_raw_parts(script_ptr as *const u8, script_len as usize) };
    let Ok(script) = std::str::from_utf8(script) else {
        store_result("invalid utf-8 script");
        return 2;
    };
    match run(script) {
        Ok(value) => {
            store_result(&value);
            0
        }
        Err(RunError::Script(message)) => {
            store_result(&message);
            1
        }
        Err(RunError::Runtime(message)) => {
            store_result(&message);
            3
        }
    }
}

#[no_mangle]
pub extern "C" fn result_ptr() -> u32 {
    RESULT.lock().expect("result lock").as_ptr() as u32
}

#[no_mangle]
pub extern "C" fn result_len() -> u32 {
    RESULT.lock().expect("result lock").len() as u32
}

enum RunError {
    Script(String),
    Runtime(String),
}

fn run(script: &str) -> Result<String, RunError> {
    let runtime = Runtime::new().map_err(|err| RunError::Runtime(err.to_string()))?;
    let context = Context::full(&runtime).map_err(|err| RunError::Runtime(err.to_string()))?;
    context.with(|ctx| {
        let call = Function::new(ctx.clone(), |name: String, args: String| -> Result<String, rquickjs::Error> {
            host_call(&name, &args).map_err(|err| rquickjs::Error::new_from_js_message("host", "call", err))
        })
        .map_err(|err| RunError::Runtime(err.to_string()))?;
        ctx.globals()
            .set("__call_tool", call)
            .map_err(|err| RunError::Runtime(err.to_string()))?;
        let log = Function::new(ctx.clone(), |text: String| {
            let bytes = text.as_bytes();
            unsafe { host_log(bytes.as_ptr() as u32, bytes.len() as u32) };
        })
        .map_err(|err| RunError::Runtime(err.to_string()))?;
        ctx.globals()
            .set("__log", log)
            .map_err(|err| RunError::Runtime(err.to_string()))?;
        let value: Value = match ctx.eval(script) {
            Ok(value) => value,
            Err(err) => {
                let exception = ctx.catch();
                let detail = exception
                    .as_object()
                    .and_then(|object| object.get::<_, String>("message").ok())
                    .unwrap_or_else(|| format!("{exception:?}"));
                return Err(RunError::Script(format!("{err}: {detail}")));
            }
        };
        let json = ctx
            .json_stringify(value)
            .map_err(|err| RunError::Runtime(err.to_string()))?;
        let json = json
            .map(|value| value.to_string())
            .transpose()
            .map_err(|err| RunError::Runtime(err.to_string()))?;
        Ok(json.unwrap_or_else(|| "null".to_string()))
    })
}

fn host_call(name: &str, args: &str) -> Result<String, String> {
    let request = serde_json::json!({
        "name": name,
        "args": serde_json::from_str::<serde_json::Value>(args).unwrap_or(serde_json::Value::Null),
    })
    .to_string();
    let ptr = alloc(request.len() as u32);
    unsafe {
        std::ptr::copy_nonoverlapping(request.as_ptr(), ptr as *mut u8, request.len());
    }
    let result_ptr = unsafe { host_tool_call(ptr, request.len() as u32) };
    dealloc(ptr, request.len() as u32);
    if result_ptr == 0 {
        return Err("host returned no result".to_string());
    }
    let len = unsafe { *(result_ptr as *const u32) } as usize;
    let bytes = unsafe { slice::from_raw_parts((result_ptr + 4) as *const u8, len) };
    String::from_utf8(bytes.to_vec()).map_err(|err| err.to_string())
}

fn store_result(value: &str) {
    let mut result = RESULT.lock().expect("result lock");
    result.clear();
    result.extend_from_slice(value.as_bytes());
}

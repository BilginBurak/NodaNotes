use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Thread-safe helper to get or initialize a global Tokio runtime for JNI calls.
fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to initialize Tokio runtime for JNI")
    })
}

#[no_mangle]
pub extern "system" fn Java_com_bubi_nodanotes_RustCore_initVault(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jstring {
    let path_str: String = match env.get_string(&path) {
        Ok(s) => s.into(),
        Err(_) => return env.new_string("Error: Invalid path string").unwrap().into_raw(),
    };

    // Use our global Tokio runtime to execute the async code synchronously
    let result = get_runtime().block_on(async {
        match noda_core::vault::VaultService::new(&path_str) {
            Ok(_) => format!("Success: Vault initialized at: {}", path_str),
            Err(e) => format!("Error: Failed to initialize vault: {}", e),
        }
    });

    env.new_string(result).unwrap().into_raw()
}

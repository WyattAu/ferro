use wasmtime::{Caller, Linker};

use crate::error::WasmHostError;
use crate::sandbox::WasmState;

pub fn add_host_functions(linker: &mut Linker<WasmState>) -> Result<(), WasmHostError> {
    linker.func_wrap("host", "log", host_log)?;
    linker.func_wrap("host", "storage_get", host_storage_get)?;
    linker.func_wrap("host", "storage_set", host_storage_set)?;
    linker.func_wrap("host", "get_config", host_get_config)?;
    Ok(())
}

fn host_log(mut caller: Caller<WasmState>, level: u32, msg_ptr: u32, msg_len: u32) {
    let msg = read_string(&mut caller, msg_ptr, msg_len).unwrap_or_default();
    caller.data_mut().log_messages.push((level, msg));
}

fn host_storage_get(mut caller: Caller<WasmState>, key_ptr: u32, key_len: u32) -> u64 {
    let key = match read_string(&mut caller, key_ptr, key_len) {
        Some(k) => k,
        None => return 0,
    };

    let data = caller.data().storage.get(&key);
    match data {
        Some(val) => {
            let len = val.len() as u64;
            if len == 0 {
                return 1;
            }
            1 | (len << 1)
        }
        None => 0,
    }
}

fn host_storage_set(mut caller: Caller<WasmState>, key_ptr: u32, key_len: u32, val_ptr: u32, val_len: u32) {
    let key = match read_string(&mut caller, key_ptr, key_len) {
        Some(k) => k,
        None => return,
    };
    let val = read_bytes(&mut caller, val_ptr, val_len).unwrap_or_default();
    caller.data_mut().storage.insert(key, val);
}

fn host_get_config(mut caller: Caller<WasmState>, key_ptr: u32, key_len: u32) -> u64 {
    let key = match read_string(&mut caller, key_ptr, key_len) {
        Some(k) => k,
        None => return 0,
    };

    let val = caller.data().host_config.get(&key);
    match val {
        Some(v) => {
            let len = v.len() as u64;
            if len == 0 {
                return 1;
            }
            1 | (len << 1)
        }
        None => 0,
    }
}

fn read_string(caller: &mut Caller<WasmState>, ptr: u32, len: u32) -> Option<String> {
    let bytes = read_bytes(caller, ptr, len)?;
    String::from_utf8(bytes).ok()
}

fn read_bytes(caller: &mut Caller<WasmState>, ptr: u32, len: u32) -> Option<Vec<u8>> {
    let memory = caller.get_export("memory")?.into_memory()?;
    let mut buf = vec![0u8; len as usize];
    memory.read(&mut *caller, ptr as usize, &mut buf).ok()?;
    Some(buf)
}

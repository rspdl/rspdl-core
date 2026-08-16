//! napi-rs boundary for the RSPDL Node.js package.

#![deny(unsafe_code)]

use napi::bindgen_prelude::AsyncTask;
use napi::{Env, Error, Result, Status, Task};
use napi_derive::napi;
use rspdl_sdk::SdkError;

enum Operation {
    Compile,
    Check,
    FindModel,
}

pub struct JsonTask {
    operation: Operation,
    request: String,
}

impl Task for JsonTask {
    type Output = String;
    type JsValue = String;

    fn compute(&mut self) -> Result<Self::Output> {
        let result = match self.operation {
            Operation::Compile => rspdl_sdk::compile_json(&self.request),
            Operation::Check => rspdl_sdk::check_json(&self.request),
            Operation::FindModel => rspdl_sdk::find_model_json(&self.request),
        };
        result.map_err(binding_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

fn binding_error(error: SdkError) -> Error {
    Error::new(Status::InvalidArg, format!("{}: {error}", error.code()))
}

#[napi(js_name = "compileJson")]
pub fn compile_json(request: String) -> AsyncTask<JsonTask> {
    AsyncTask::new(JsonTask {
        operation: Operation::Compile,
        request,
    })
}

#[napi(js_name = "checkJson")]
pub fn check_json(request: String) -> AsyncTask<JsonTask> {
    AsyncTask::new(JsonTask {
        operation: Operation::Check,
        request,
    })
}

#[napi(js_name = "findModelJson")]
pub fn find_model_json(request: String) -> AsyncTask<JsonTask> {
    AsyncTask::new(JsonTask {
        operation: Operation::FindModel,
        request,
    })
}

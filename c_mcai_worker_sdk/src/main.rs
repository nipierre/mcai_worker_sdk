mod constants;
#[cfg(feature = "media")]
mod filters;
mod parameters;
mod process_return;
#[cfg(feature = "media")]
mod stream_descriptors;
mod worker;

#[macro_use]
extern crate serde_derive;

use crate::parameters::CWorkerParameters;
use crate::worker::*;
use mcai_worker_sdk::{
  debug, job::JobResult, start_worker, McaiChannel, MessageEvent, Result, Version,
};
#[cfg(feature = "media")]
use mcai_worker_sdk::{FormatContext, Frame, ProcessResult, StreamDescriptor};
#[cfg(feature = "media")]
use std::sync::{mpsc::Sender, Arc, Mutex};

#[macro_export]
macro_rules! get_c_string {
  ($name:expr) => {
    if $name.is_null() {
      "".to_string()
    } else {
      std::str::from_utf8_unchecked(std::ffi::CStr::from_ptr($name).to_bytes()).to_string()
    }
  };
}

#[derive(Clone, Debug, Default)]
struct CWorkerEvent {
  #[cfg(feature = "media")]
  result: Option<Arc<Mutex<Sender<ProcessResult>>>>,
}

impl MessageEvent<CWorkerParameters> for CWorkerEvent {
  fn get_name(&self) -> String {
    get_worker_function_string_value(constants::GET_NAME_FUNCTION)
  }

  fn get_short_description(&self) -> String {
    get_worker_function_string_value(constants::GET_SHORT_DESCRIPTION_FUNCTION)
  }

  fn get_description(&self) -> String {
    get_worker_function_string_value(constants::GET_DESCRIPTION_FUNCTION)
  }

  fn get_version(&self) -> Version {
    let version = get_worker_function_string_value(constants::GET_VERSION_FUNCTION);
    Version::parse(&version).unwrap_or_else(|_| {
      panic!(
        "unable to parse version {} (please use SemVer format)",
        version
      )
    })
  }

  fn init(&mut self) -> Result<()> {
    call_optional_worker_init()
  }

  #[cfg(feature = "media")]
  fn init_process(
    &mut self,
    parameters: CWorkerParameters,
    format_context: Arc<Mutex<FormatContext>>,
    result: Arc<Mutex<Sender<ProcessResult>>>,
  ) -> Result<Vec<StreamDescriptor>> {
    self.result = Some(result);
    call_worker_init_process(parameters, format_context)
  }

  #[cfg(feature = "media")]
  fn process_frame(
    &mut self,
    job_result: JobResult,
    stream_index: usize,
    frame: Frame,
  ) -> Result<ProcessResult> {
    call_worker_process_frame(job_result, stream_index, frame)
  }

  #[cfg(feature = "media")]
  fn ending_process(&mut self) -> Result<()> {
    let ending_process_result = call_worker_ending_process();

    if let Some(result) = &self.result {
      result
        .lock()
        .unwrap()
        .send(ProcessResult::end_of_process())
        .unwrap();
    }

    ending_process_result
  }

  fn process(
    &self,
    channel: Option<McaiChannel>,
    parameters: CWorkerParameters,
    job_result: JobResult,
  ) -> Result<JobResult> {
    debug!("Process job: {}", job_result.get_job_id());
    let process_return = call_worker_process(job_result.clone(), parameters, channel)?;
    debug!("Returned: {:?}", process_return);
    process_return.as_result(job_result)
  }
}

fn main() {
  start_worker(CWorkerEvent::default());
}

#[test]
pub fn test_c_binding_worker_info() {
  use mcai_worker_sdk::worker::ParameterType;

  let worker_event = CWorkerEvent::default();
  let name = worker_event.get_name();
  let short_description = worker_event.get_short_description();
  let description = worker_event.get_description();
  let version = worker_event.get_version().to_string();

  assert_eq!(name, "my_c_worker".to_string());
  assert_eq!(short_description, "My C Worker".to_string());
  assert_eq!(
    description,
    "This is my long description \nover multilines".to_string()
  );
  assert_eq!(version, "0.1.0".to_string());

  let parameters = get_worker_parameters();
  assert_eq!(3, parameters.len());

  assert_eq!("my_parameter".to_string(), parameters[0].identifier);
  assert_eq!("My parameter".to_string(), parameters[0].label);
  assert_eq!(1, parameters[0].kind.len());
  assert!(!parameters[0].required);

  let parameter_kind =
    serde_json::to_string(&parameters[0].kind[0]).expect("cannot serialize parameter kind");
  let expected_kind =
    serde_json::to_string(&ParameterType::String).expect("cannot serialize parameter kind");
  assert_eq!(expected_kind, parameter_kind);

  assert_eq!("source_path".to_string(), parameters[1].identifier);
  assert_eq!("Source path".to_string(), parameters[1].label);
  assert_eq!(1, parameters[1].kind.len());
  assert!(parameters[1].required);

  let parameter_kind =
    serde_json::to_string(&parameters[1].kind[0]).expect("cannot serialize parameter kind");
  let expected_kind =
    serde_json::to_string(&ParameterType::String).expect("cannot serialize parameter kind");
  assert_eq!(expected_kind, parameter_kind);

  assert_eq!("destination_path".to_string(), parameters[2].identifier);
  assert_eq!("Destination path".to_string(), parameters[2].label);
  assert_eq!(1, parameters[2].kind.len());
  assert!(parameters[2].required);

  let parameter_kind =
    serde_json::to_string(&parameters[2].kind[0]).expect("cannot serialize parameter kind");
  let expected_kind =
    serde_json::to_string(&ParameterType::String).expect("cannot serialize parameter kind");
  assert_eq!(expected_kind, parameter_kind);
}

#[cfg(test)]
use mcai_worker_sdk::job::{Job, JobStatus};

#[test]
pub fn test_init() {
  let mut c_worker_event = CWorkerEvent::default();
  let result = c_worker_event.init();
  assert!(result.is_ok());
}

#[test]
#[cfg(feature = "media")]
pub fn test_ending_process() {
  let mut c_worker_event = CWorkerEvent::default();
  let result = c_worker_event.ending_process();
  assert!(result.is_ok());
}

#[test]
pub fn test_process() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      {
        "id": "path",
        "type": "string",
        "value": "/path/to/file"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();
  let job_result = JobResult::new(job.job_id);
  let parameters = job.get_parameters().unwrap();

  let result = CWorkerEvent::default().process(None, parameters, job_result);
  assert!(result.is_ok());
  let job_result = result.unwrap();
  assert_eq!(job_result.get_job_id(), 123);
  assert_eq!(job_result.get_status(), &JobStatus::Completed);
  assert_eq!(
    job_result.get_destination_paths(),
    &vec!["/path/out.mxf".to_string()]
  );
}

#[test]
pub fn test_failing_process() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      {
        "id": "not_the_expected_path_parameter",
        "type": "string",
        "value": "value"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();
  let job_result = JobResult::new(job.job_id);
  let parameters = job.get_parameters().unwrap();

  let result = CWorkerEvent::default().process(None, parameters, job_result);
  assert!(result.is_err());
  let _message_error = result.unwrap_err();
}

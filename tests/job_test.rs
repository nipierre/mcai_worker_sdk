extern crate amqp_worker;
extern crate assert_matches;

use amqp_worker::job::Job;
use amqp_worker::MessageError;
use assert_matches::assert_matches;

#[test]
fn test_new_job_empty_message() {
  let message = "";
  let result = Job::new(message);
  assert!(result.is_err());
  let error = result.unwrap_err();
  assert_matches!(error, MessageError::RuntimeError(_));
}

#[test]
fn test_new_job_invalid_message() {
  let message = "{}";
  let result = Job::new(message);
  assert!(result.is_err());
  let error = result.unwrap_err();
  assert_matches!(error, MessageError::RuntimeError(_));
}

#[test]
fn test_new_job_invalid_parameter() {
  let message = "{\
    \"job_id\": 123,\
    \"parameters\": [\
      { \"key\":\"value\" },\
    ]\
  }";
  let result = Job::new(message);
  assert!(result.is_err());
  let error = result.unwrap_err();
  assert_matches!(error, MessageError::RuntimeError(_));
}

#[test]
fn test_new_job() {
  let message = "{\
    \"job_id\": 123,\
    \"parameters\": [\
      { \"id\":\"string_parameter\",\
        \"type\":\"string\",\
        \"default\":\"default_value\",\
        \"value\":\"real_value\" },\
      { \"id\":\"boolean_parameter\",\
        \"type\":\"boolean\",\
        \"default\": false,\
        \"value\": true },\
      { \"id\":\"integer_parameter\",\
        \"type\":\"integer\",\
        \"default\": 123456,\
        \"value\": 654321 },\
      { \"id\":\"credential_parameter\",\
        \"type\":\"credential\",\
        \"default\":\"default_credential_key\",\
        \"value\":\"credential_key\" },\
      { \"id\":\"array_of_string_parameter\",\
        \"type\":\"array_of_strings\",\
        \"default\": [\"default_value\"],\
        \"value\": [\"real_value\"] }\
    ]
  }";

  let result = Job::new(message);
  assert!(result.is_ok());
  let job = result.unwrap();
  assert_eq!(123, job.job_id);

  let optional_string = job.get_string_parameter("string_parameter");
  assert!(optional_string.is_some());
  let string_value = optional_string.unwrap();
  assert_eq!("real_value".to_string(), string_value);

  let optional_boolean = job.get_boolean_parameter("boolean_parameter");
  assert!(optional_boolean.is_some());
  let boolean_value = optional_boolean.unwrap();
  assert_eq!(true, boolean_value);

  let optional_integer = job.get_integer_parameter("integer_parameter");
  assert!(optional_integer.is_some());
  let integer_value = optional_integer.unwrap();
  assert_eq!(654321, integer_value);

  let optional_credential = job.get_credential_parameter("credential_parameter");
  assert!(optional_credential.is_some());
  let credential_value = optional_credential.unwrap();
  assert_eq!("credential_key", credential_value.key);

  let option_array = job.get_array_of_strings_parameter("array_of_string_parameter");
  assert!(option_array.is_some());
  let array_of_values = option_array.unwrap();
  assert_eq!(1, array_of_values.len());
  assert_eq!("real_value".to_string(), array_of_values[0]);
}


#[test]
fn test_check_requirements() {
  let message = "{\
    \"job_id\": 123,\
    \"parameters\": [\
      { \"id\":\"requirements\",\
        \"type\":\"requirements\",\
        \"value\": {\
          \"paths\": [\
            \"./tests/job_test.rs\"\
          ]\
        }\
      }\
    ]\
  }";

  let result = Job::new(message);
  assert!(result.is_ok());
  let job = result.unwrap();
  assert_eq!(123, job.job_id);

  let requirement_result = job.check_requirements();
  assert!(requirement_result.is_ok());
}

#[test]
fn test_check_invalid_requirements() {
  let message = "{\
    \"job_id\": 123,\
    \"parameters\": [\
      { \"id\":\"requirements\",\
        \"type\":\"requirements\",\
        \"value\": {\
          \"paths\": [\
            \"nonexistent_file\"\
          ]\
        }\
      }\
    ]\
  }";

  let result = Job::new(message);
  assert!(result.is_ok());
  let job = result.unwrap();
  assert_eq!(123, job.job_id);

  let requirement_result = job.check_requirements();
  assert!(requirement_result.is_err());
  let error = requirement_result.unwrap_err();
  assert_matches!(error, MessageError::RequirementsError(_));
}

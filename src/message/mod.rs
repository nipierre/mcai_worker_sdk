mod helpers;

use crate::{
  config::{get_amqp_completed_queue, get_amqp_error_queue},
  job::{JobResult, JobStatus},
  MessageError, MessageEvent,
};

use futures::future::Future;
use lapin::{message::Delivery, options::*, BasicProperties, Channel};
use serde_json::Value;

pub fn process_message<ME: MessageEvent>(
  message_event: &'static ME,
  message: Delivery,
  channel: &Channel,
) {
  let count = helpers::get_message_death_count(&message);
  let message_data = std::str::from_utf8(&message.data).unwrap();
  info!(
    "got message: {} (iteration: {})",
    message_data,
    count.unwrap_or(0)
  );

  match MessageEvent::process(message_event, message_data) {
    Ok(job_result) => {
      info!(target: &job_result.job_id.to_string(), "Completed");
      let msg = json!(job_result);

      publish_completed_job(channel, message, job_result, msg);
    }
    Err(error) => match error {
      MessageError::RequirementsError(details) => {
        publish_missing_requirements(channel, message, &details);
      }
      MessageError::NotImplemented() => {
        publish_not_implemented(channel, message);
      }
      MessageError::ProcessingError(job_result) => {
        publish_processing_error(channel, message, job_result);
      }
      MessageError::RuntimeError(error_message) => {
        publish_runtime_error(channel, message, &error_message);
      }
    },
  }
}

fn publish_completed_job(channel: &Channel, message: Delivery, job_result: JobResult, msg: Value) {
  let amqp_completed_queue = get_amqp_completed_queue();
  let result = channel
    .basic_publish(
      "", // exchange
      &amqp_completed_queue,
      msg.to_string().as_str().as_bytes().to_vec(),
      BasicPublishOptions::default(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok();

  if result {
    if let Err(msg) = channel
      .basic_ack(message.delivery_tag, false /*not requeue*/)
      .wait()
    {
      error!(target: &job_result.job_id.to_string(), "Unable to ack message {:?}", msg);
    }
  } else if let Err(msg) = channel
    .basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
    .wait()
  {
    error!(target: &job_result.job_id.to_string(), "Unable to reject message {:?}", msg);
  }
}

fn publish_missing_requirements(channel: &Channel, message: Delivery, details: &str) {
  debug!("{}", details);
  if let Err(msg) = channel
    .basic_reject(message.delivery_tag, BasicRejectOptions::default())
    .wait()
  {
    error!("Unable to reject message {:?}", msg);
  }
}

fn publish_not_implemented(channel: &Channel, message: Delivery) {
  error!("Not implemented feature");
  if let Err(msg) = channel
    .basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
    .wait()
  {
    error!("Unable to reject message {:?}", msg);
  }
}

fn publish_processing_error(channel: &Channel, message: Delivery, job_result: JobResult) {
  let amqp_error_queue = get_amqp_error_queue();

  error!(target: &job_result.job_id.to_string(), "Job returned in error: {:?}", job_result.parameters);
  let content = json!(JobResult {
    job_id: job_result.job_id,
    status: JobStatus::Error,
    parameters: job_result.parameters,
  });
  if channel
    .basic_publish(
      "", // exchange
      &amqp_error_queue,
      content.to_string().as_str().as_bytes().to_vec(),
      BasicPublishOptions::default(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok()
  {
    if let Err(msg) = channel
      .basic_ack(message.delivery_tag, false /*not requeue*/)
      .wait()
    {
      error!(target: &job_result.job_id.to_string(), "Unable to ack message {:?}", msg);
    }
  } else if let Err(msg) = channel
    .basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
    .wait()
  {
    error!(target: &job_result.job_id.to_string(), "Unable to reject message {:?}", msg);
  }
}

fn publish_runtime_error(channel: &Channel, message: Delivery, details: &str) {
  let amqp_error_queue = get_amqp_error_queue();

  error!("An error occurred: {:?}", details);
  let content = json!({
    "status": "error",
    "message": details
  });
  if channel
    .basic_publish(
      "", // exchange
      &amqp_error_queue,
      content.to_string().as_str().as_bytes().to_vec(),
      BasicPublishOptions::default(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok()
  {
    if let Err(msg) = channel
      .basic_ack(message.delivery_tag, false /*not requeue*/)
      .wait()
    {
      error!("Unable to ack message {:?}", msg);
    }
  } else if let Err(msg) = channel
    .basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
    .wait()
  {
    error!("Unable to reject message {:?}", msg);
  }
}

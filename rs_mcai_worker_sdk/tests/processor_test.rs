#[macro_use]
extern crate serde_derive;

use mcai_worker_sdk::message_exchange::ResponseMessage;
use mcai_worker_sdk::{
  job::{Job, JobResult},
  message_exchange::{ExternalExchange, LocalExchange, OrderMessage},
  processor::Processor,
  JsonSchema, McaiChannel, MessageEvent, Result,
};
use std::sync::{Arc, Mutex};

#[test]
fn processor() {
  struct Worker {}

  #[derive(Clone, Debug, Deserialize, JsonSchema)]
  pub struct WorkerParameters {}

  impl MessageEvent<WorkerParameters> for Worker {
    fn get_name(&self) -> String {
      "Test Worker".to_string()
    }

    fn get_short_description(&self) -> String {
      "The Worker defined in unit tests".to_string()
    }

    fn get_description(&self) -> String {
      "Mock a Worker to realise tests around SDK".to_string()
    }

    fn get_version(&self) -> semver::Version {
      semver::Version::parse("1.2.3").unwrap()
    }

    fn process(
      &self,
      channel: Option<McaiChannel>,
      _parameters: WorkerParameters,
      job_result: JobResult,
    ) -> Result<JobResult>
    where
      Self: std::marker::Sized,
    {
      println!("NEW JOB");
      println!("{}", channel.is_some());
      Ok(job_result)
    }
  }

  let mut local_exchange = LocalExchange::new();
  let local_exchange_ref = Arc::new(Mutex::new(local_exchange.clone()));
  let processor = Processor::new(local_exchange_ref);

  let worker = Worker {};

  std::thread::spawn(move || {
    assert!(processor.run(worker).is_ok());
  });

  let job = Job::new(r#"{ "job_id": 666, "parameters": [] }"#).unwrap();

  local_exchange.send_order(OrderMessage::Job(job)).unwrap();
  local_exchange.send_order(OrderMessage::Stop).unwrap();

  let response = local_exchange.next_response().unwrap();
  assert_eq!(ResponseMessage::Completed, response.unwrap());
}
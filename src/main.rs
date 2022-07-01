use std::fmt::{Debug, Display};

use tokio::task::JoinError;
use zero2prod::configuration::get_configuration;
use zero2prod::issue_delivery_worker::run_worker_until_stopped;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("zero2prod".to_string(), "info".to_string(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration file");
    let application = Application::build(configuration.clone()).await?;
    let worker = run_worker_until_stopped(configuration);
    let application_task = tokio::spawn(application.run_until_stopped());
    let worker_task = tokio::spawn(worker);

    // tokio::select! gets both tasks to make progress concurrently.
    // tokio::select! returns when one of the two tasks completes or errors out
    // If one runs all async expressions on the current task, expressions run concurrently but not in parallel.
    // Therefore, if one branch blocks the thread, all other expressions will be unable to continue
    // To enable parallelism, spawn each async expression to a separate thread
    // and pass the join handle to tokio::select!
    tokio::select! {
        o = application_task => report_exit("API", o),
        o = worker_task => report_exit("Background worker", o),
    };

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        }
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} task failed to complete",
                task_name
            )
        }
    }
}

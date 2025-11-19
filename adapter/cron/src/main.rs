use std::str::FromStr;
use async_trait::async_trait;
use chrono::{Local, Utc};
use cron::Schedule;
use base::extract_flow_setting_field;
use base::runner::{ServerContext, ServerRunner};
use base::traits::{IdentifiableFlow, LoadConfig, Server};

#[derive(Default)]
struct Cron {
}

#[derive(Clone)]
struct CronConfig {}

impl LoadConfig for CronConfig {
    fn load() -> Self {
        Self {}
    }
}

#[tokio::main]
async fn main() {
    let server = Cron::default();
    let runner = ServerRunner::new(server).await.unwrap();
    runner.serve().await.unwrap();
}

struct Time {
    utc: Utc
}

impl IdentifiableFlow for Time {
    fn identify(&self, flow: &tucana::shared::ValidationFlow) -> bool {
        let Some(minute) = extract_flow_setting_field(&flow.settings, "CRON_MINUTE", "minute") else {
            return false;
        };
        let Some(hour) = extract_flow_setting_field(&flow.settings, "CRON_MOUR", "hour") else {
            return false;
        };
        let Some(dom) = extract_flow_setting_field(&flow.settings, "CRON_DAY_OF_MONTH", "day_of_month") else {
            return false;
        };
        let Some(month) = extract_flow_setting_field(&flow.settings, "CRON_MONTH", "month") else {
            return false;
        };
        let Some(dow) = extract_flow_setting_field(&flow.settings, "CRON_DAY_OF_WEEK", "day_of_week") else {
            return false;
        };

        let expression = format!("{} {} {} {} {}", minute, hour, dom, month, dow);




        todo!()
    }
}

#[async_trait]
impl Server<CronConfig> for Cron {
    async fn init(&mut self, _ctx: &ServerContext<CronConfig>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run(&mut self, ctx: &ServerContext<CronConfig>) -> anyhow::Result<()> {
        let expression = "0 * * * * *";
        let schedule = Schedule::from_str(expression).expect("Failed to parse CRON expression");

        loop {
            let now = Utc::now();
            if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                let until_next = next - now;
                tokio::time::sleep(until_next.to_std()?).await;
                println!(
                    "Running every minute. Current time: {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S")
                );
            }
        }
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &ServerContext<CronConfig>) -> anyhow::Result<()> {
        Ok(())
    }
}

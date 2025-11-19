use async_trait::async_trait;
use base::extract_flow_setting_field;
use base::runner::{ServerContext, ServerRunner};
use base::store::FlowIdentifyResult;
use base::traits::{IdentifiableFlow, LoadConfig, Server};
use chrono::{DateTime, Datelike, Timelike, Utc};
use cron::Schedule;
use std::str::FromStr;

#[derive(Default)]
struct Cron {}

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
    now: DateTime<Utc>,
}

impl IdentifiableFlow for Time {
    fn identify(&self, flow: &tucana::shared::ValidationFlow) -> bool {
        let Some(minute) = extract_flow_setting_field(&flow.settings, "CRON_MINUTE", "minute")
        else {
            return false;
        };
        let Some(hour) = extract_flow_setting_field(&flow.settings, "CRON_HOUR", "hour") else {
            return false;
        };
        let Some(dom) =
            extract_flow_setting_field(&flow.settings, "CRON_DAY_OF_MONTH", "day_of_month")
        else {
            return false;
        };
        let Some(month) = extract_flow_setting_field(&flow.settings, "CRON_MONTH", "month") else {
            return false;
        };
        let Some(dow) =
            extract_flow_setting_field(&flow.settings, "CRON_DAY_OF_WEEK", "day_of_week")
        else {
            return false;
        };

        let expression = format!("* {} {} {} {} {}", minute, hour, dom, month, dow);
        let schedule = Schedule::from_str(expression.as_str()).unwrap();
        let next = schedule.upcoming(Utc).next().unwrap();

        self.now.year() == next.year()
            && self.now.month() == next.month()
            && self.now.day() == next.day()
            && self.now.hour() == next.hour()
            && self.now.minute() == next.minute()
    }
}

#[async_trait]
impl Server<CronConfig> for Cron {
    async fn init(&mut self, _ctx: &ServerContext<CronConfig>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run(&mut self, ctx: &ServerContext<CronConfig>) -> anyhow::Result<()> {
        let expression = "0 * * * * *";
        let schedule = Schedule::from_str(expression)?;
        let pattern = "*.*.CRON.*";

        loop {
            let now = Utc::now();
            if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                let until_next = next - now;
                tokio::time::sleep(until_next.to_std()?).await;

                let time = Time { now };
                match ctx
                    .adapter_store
                    .get_possible_flow_match(pattern.to_string(), time)
                    .await
                {
                    FlowIdentifyResult::None => {}
                    FlowIdentifyResult::Single(flow) => {
                        ctx.adapter_store
                            .validate_and_execute_flow(flow, None, false)
                            .await;
                    }
                    FlowIdentifyResult::Multiple(flows) => {
                        for flow in flows {
                            ctx.adapter_store
                                .validate_and_execute_flow(flow, None, false)
                                .await;
                        }
                    }
                }
            }
        }
    }

    async fn shutdown(&mut self, _ctx: &ServerContext<CronConfig>) -> anyhow::Result<()> {
        Ok(())
    }
}

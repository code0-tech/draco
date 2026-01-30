use async_trait::async_trait;
use base::runner::{ServerContext, ServerRunner};
use base::store::FlowIdentifyResult;
use base::traits::{IdentifiableFlow, LoadConfig, Server};
use chrono::{DateTime, Datelike, Timelike, Utc};
use cron::Schedule;
use std::str::FromStr;
use tucana::shared::ValidationFlow;
use tucana::shared::value::Kind;

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

fn extract_flow_setting_field(flow: &ValidationFlow, name: &str) -> Option<String> {
    flow.settings
        .iter()
        .find(|s| s.flow_setting_id == name)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| v.kind.as_ref())
        .and_then(|k| match k {
            Kind::StringValue(s) => Some(s.clone()),
            _ => None,
        })
}

impl IdentifiableFlow for Time {
    fn identify(&self, flow: &tucana::shared::ValidationFlow) -> bool {
        let Some(minute) = extract_flow_setting_field(flow, "CRON_MINUTE") else {
            return false;
        };
        let Some(hour) = extract_flow_setting_field(flow, "CRON_HOUR") else {
            return false;
        };
        let Some(dom) = extract_flow_setting_field(flow, "CRON_DAY_OF_MONTH") else {
            return false;
        };
        let Some(month) = extract_flow_setting_field(flow, "CRON_MONTH") else {
            return false;
        };
        let Some(dow) = extract_flow_setting_field(flow, "CRON_DAY_OF_WEEK") else {
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
        log::info!("Starting Cron adapter");
        let expression = "0 * * * * *";
        let schedule = Schedule::from_str(expression)?;
        let pattern = "CRON.<";

        loop {
            let now = Utc::now();
            log::info!("Schedlued: {:?}", now);
            if let Some(next) = schedule.upcoming(Utc).take(1).next() {
                let until_next = next - now;
                tokio::time::sleep(until_next.to_std()?).await;

                let time = Time { now };
                match ctx
                    .adapter_store
                    .get_possible_flow_match(pattern.to_string(), time)
                    .await
                {
                    FlowIdentifyResult::None => {
                        log::debug!("No Flow identified for this schedule");
                    }
                    FlowIdentifyResult::Single(flow) => {
                        log::debug!("One Flow identified for this schedule");
                        ctx.adapter_store
                            .validate_and_execute_flow(flow, None)
                            .await;
                    }
                    FlowIdentifyResult::Multiple(flows) => {
                        log::debug!("Multiple Flows identified for this schedule");
                        for flow in flows {
                            ctx.adapter_store
                                .validate_and_execute_flow(flow, None)
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

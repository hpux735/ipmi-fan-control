#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate anyhow;

use args::Command;
use clap::Parser;
use ipmi::{Cmd, Ipmi, IpmiTool};
use log::{error, info};
use std::ops::RangeInclusive;
use tokio::time::{self, Duration};
use pretty_env_logger;
use crate::fan_curve::fan_speed;

use ringbuf::HeapRb;

mod args;
mod ipmi;
mod fan_curve;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    pretty_env_logger::init();

    let args = args::Args::parse();

    let tool = IpmiTool::new(Box::new(Cmd::new()));

    info!("Starting IPMI Fan Control tool!");

    match args.command {
        Command::Auto(a) => {
            let mut interval = a.interval;
            if !RangeInclusive::new(5, 120).contains(&interval) {
                interval = 5;
                info!("invalid interval, interval set to 5");
            }

            let mut ring = HeapRb::new(a.samples_average as usize);

            let mut threshold = a.threshold;
            if !RangeInclusive::new(60, 100).contains(&threshold) {
                threshold = 70;
                info!("invalid threshold, threshold set to {}", threshold);
            }

            info!(
                "auto mode start, interval: {}, threshold: {}",
                interval, threshold
            );

            let mut interval = time::interval(Duration::from_secs(interval));

            let mut last_speed = 0xff;

            loop {
                interval.tick().await;

                if let Ok(temperature) = tool.get_cpu_temperature() {
                    // transfer temperature to fan speed
                    let mut speed = fan_speed(temperature, &mut ring);

                    if temperature >= threshold {
                        speed = 100;
                        info!("temperature reach threshold {}", temperature);
                    }

                    if speed > 100 {
                        speed = 100;
                    }

                    if last_speed != speed {
                        match tool.set_fan_speed(speed) {
                            Ok(_) => {
                                last_speed = speed;
                                info!("temperature: {}, set fan speed to {}", temperature, speed);
                            }
                            Err(e) => error!("failed to set fan speed: {}", e),
                        }
                    }
                } else {
                    error!("failed to get cpu temperature");
                }
            }
        }
        Command::Fixed { value } => {
            let mut v = value;
            if v > 100 {
                v = 100;
            }
            info!("fixed mode, set fan speed to {}", v);
            if let Err(e) = tool.set_fan_speed(v) {
                error!("set fan speed, error: {}", e);
            }
        }
        Command::Info => match tool.get_info_fan_temp() {
            Ok(info) => {
                println!("{}", info);
            }
            Err(err) => {
                error!("get info error: {}", err);
            }
        },
    }
}

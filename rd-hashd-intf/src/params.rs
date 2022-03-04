// Copyright (c) Facebook, Inc. and its affiliates.
use anyhow::Result;
use serde::{Deserialize, Serialize};

use rd_util::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PidParams {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
}

const PARAMS_DOC: &str = "\
//
// rd-hashd runtime parameters
//
// All parameters can be updated while running and will be applied immediately.
//
// rd-hashd keeps calculating SHA1s of different parts of testfiles using
// concurrent worker threads. The testfile indices and hash sizes are
// determined using truncated normal distributions which gradually transforms
// to uniform distributions as their standard deviations increase.
//
// All durations are in seconds and memory bytes. A _frac field should be <=
// 1.0 and specifies a sub-proportion of some other value. A _ratio field is
// similar but may be greater than 1.0.
//
// The concurrency level is modulated using two PID controllers to target the
// specified latency and RPS so that neither is exceeded. The total number
// of concurrent threads is limited by `concurrency_max`.
//
// The total size of testfiles is set up during startup and can't be changed
// online. However, the portion which is actively used by rd-hashd can be
// scaled down with `file_total_frac`.
//
// Anonymous memory accesses can be specified in two different ways:
// 1) Anonymous memory total and access sizes can be configured as to be proportional to
// file access sizes.
// 2) A histogram from which the memory access pattern will be sampled from can be
// passed in.
//
// The total footprint for file accesses is scaled between
// `file_addr_rps_base_frac` and 1.0 linearly if the current RPS is lower than
// `rps_max`. If `rps_max` is 0, access footprint scaling is disabled. Anon
// footprint is scaled the same way between 'anon_addr_rps_base_frac' and 1.0.
//
// Worker threads will sleep according to the sleep duration distribution and
// their CPU consumption can be scaled up and down using `cpu_ratio`.
//
//  control_period: PID control period, best left alone
//  concurrency_max: Maximum number of worker threads
//  lat_target_pct: Latency target percentile
//  lat_target: Latency target
//  rps_target: Request-per-second target
//  rps_max: Reference maximum RPS, used to scale the amount of used memory
//  chunk_pages: Memory access chunk size in pages
//  mem_frac: Memory footprint scaling factor - [0.0, 1.0]
//  file_frac: Page cache proportion of memory footprint - [0.0, 1.0]
//  file_size_mean: File access size average
//  file_size_stdev_ratio: Standard deviation of file access sizes
//  file_addr_stdev_ratio: Standard deviation of file access addresses
//  file_addr_rps_base_frac: Memory scaling starting point for file accesses
//  file_write_frac: The proportion of writes in file accesses
//  anon_size_ratio: Anon access size average - 1.0 means equal as file accesses (ignored if anon_hisogram is provided)
//  anon_size_stdev_ratio: Standard deviation of anon access sizes (ignored if anon_hisogram is provided)
//  anon_addr_stdev_ratio: Standard deviation of anon access addresses (ignored if anon_hisogram is provided)
//  anon_addr_rps_base_frac: Memory scaling starting point for anon accesses
//  anon_write_frac: The proportion of writes in anon accesses
//  anon_histogram: List of ints where the index of each element represents
//    a memory access chunk (see chunk_pages) and the value is the weight
//    which determines how frequently that chunk is accessed
//  sleep_mean: Worker sleep duration average
//  sleep_stdev_ratio: Standard deviation of sleep duration distribution
//  cpu_ratio: CPU usage scaling - 1.0 hashes the same number of bytes as accessed
//  log_bps: Log write bps at rps_max
//  fake_cpu_load: Sleep equivalent time durations instead of calculating SHA1s
//  acc_dist_slots: Access distribution report slots - 0 disables
//  lat_pid: PID controller parameters for latency convergence
//  rps_pid: PID controller parameters for RPS convergence
//
";

/// Dispatch and hash parameters, can be adjusted dynamially.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Params {
    pub control_period: f64,
    pub concurrency_max: u32,
    pub lat_target_pct: f64,
    pub lat_target: f64,
    pub rps_target: u32,
    pub rps_max: u32,
    pub mem_frac: f64,
    pub chunk_pages: usize,
    pub file_frac: f64,
    pub file_size_mean: usize,
    pub file_size_stdev_ratio: f64,
    pub file_addr_stdev_ratio: f64,
    pub file_addr_rps_base_frac: f64,
    pub file_write_frac: f64,
    pub anon_size_ratio: f64,
    pub anon_size_stdev_ratio: f64,
    pub anon_addr_stdev_ratio: f64,
    pub anon_addr_rps_base_frac: f64,
    pub anon_write_frac: f64,
    pub sleep_mean: f64,
    pub sleep_stdev_ratio: f64,
    pub cpu_ratio: f64,
    pub log_bps: u64,
    pub fake_cpu_load: bool,
    pub acc_dist_slots: usize,
    pub lat_pid: PidParams,
    pub rps_pid: PidParams,
    pub anon_histogram: Vec<u64>,
}

impl Params {
    pub const FILE_FRAC_MIN: f64 = 0.001;

    pub fn log_padding(&self) -> u64 {
        if self.rps_max > 0 {
            (self.log_bps as f64 / self.rps_max as f64).round() as u64
        } else {
            0
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self {
            control_period: 1.0,
            concurrency_max: 65536,
            lat_target_pct: 0.95,
            lat_target: 75.0 * MSEC,
            rps_target: 65536,
            rps_max: 0,
            chunk_pages: 25,
            mem_frac: 0.80,
            file_frac: 0.25,
            file_size_mean: 1258291,
            file_size_stdev_ratio: 0.45,
            file_addr_stdev_ratio: 0.215,
            file_addr_rps_base_frac: 0.5,
            file_write_frac: 0.0,
            anon_size_ratio: 2.3,
            anon_size_stdev_ratio: 0.45,
            anon_addr_stdev_ratio: 0.235,
            anon_addr_rps_base_frac: 0.5,
            anon_write_frac: 0.3,
            sleep_mean: 20.0 * MSEC,
            sleep_stdev_ratio: 0.33,
            cpu_ratio: 0.93,
            log_bps: 1100794,
            fake_cpu_load: false,
            acc_dist_slots: 0,
            lat_pid: PidParams {
                kp: 0.1,
                ki: 0.01,
                kd: 0.01,
            },
            rps_pid: PidParams {
                kp: 0.25,
                ki: 0.01,
                kd: 0.01,
            },
            anon_histogram: Vec::new(),
        }
    }
}

impl JsonLoad for Params {
    fn loaded(&mut self, _prev: Option<&mut Self>) -> Result<()> {
        self.file_frac = self.file_frac.max(Self::FILE_FRAC_MIN);
        Ok(())
    }
}

impl JsonSave for Params {
    fn preamble() -> Option<String> {
        Some(PARAMS_DOC.to_string())
    }
}

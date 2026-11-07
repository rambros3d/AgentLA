#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod generator;
pub mod scope;
pub mod servo;

pub use generator::{calculate_slice_params as calculate_generator_params, duty_to_level, PwmTiming, PwmTimingMath};
pub use scope::{
    compute_metrics, compute_multi_channel_metrics, compute_strided_metrics, find_trigger,
    ChannelMetrics, OscilloscopeMetrics, TriggerDetector, TriggerType,
};
pub use servo::{
    angle_to_pulse, clamp_pulse_us, continuous_pulse, level_counts, slice_conf,
    slice_conf_with_clk, ServoMath, SliceConf, SERVO_50HZ,
};

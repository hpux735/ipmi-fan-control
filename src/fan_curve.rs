use log::debug;
use ringbuf::{traits::*, HeapRb};

pub fn fan_speed(in_temperature: u16, ring: &mut HeapRb<u16>) -> u16 {
    // Do averaging of the temperature over some amount of time
    // this is accomplished by adding samples into the ring buffer
    // and computing the mean of those samples
    ring.push_overwrite(in_temperature);

    let temperatures: Vec<f32> = ring.iter().map(|v| *v as f32).collect();
    let temperature = temperatures
        .iter()
        .fold(0f32, |a, v| a + v) / temperatures.len() as f32;

    debug!("Input temperature: {}, averaged: {}", in_temperature, temperature);

    let curve = vec![
        (  0.0,   0),
        ( 10.0,   1),
        ( 20.0,   2),
        ( 30.0,   3),
        ( 40.0,   5),
        ( 50.0,  10),
        ( 60.0,  20),
        ( 70.0,  32),
        ( 80.0,  50),
        ( 90.0,  70),
        (100.0, 100),
    ];

    // Clip any temperature outside the curve to the endpoints
    if temperature > 80.0 { return 100 }

    // Find the interpolation points
    let mut bottom = (0.0, 0.0);
    let mut top = (70.0, 0.0);

    for (temp, speed) in curve {
        if temp > temperature {
            top = (temp as f32, speed as f32);
            break;
        } else {
            bottom = (temp as f32, speed as f32)
        }
    }

    // Get the ratio between interpolation points
    let midpoint = (temperature as f32 - bottom.0) / (top.0 - bottom.0);
    let delta = top.1 - bottom.1;

    let result = (bottom.1 + delta * midpoint) as u16;

    debug!(
        "Temperature {} is between {:?} and {:?}.  Midpoint: {}, delta: {}, scale: {} result: {}",
        temperature, bottom, top, midpoint, delta, delta * midpoint, result
    );

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fan_curve() {
        let mut ring = HeapRb::new(1);
        assert_eq!(fan_speed(  0, &mut ring),   0);
        assert_eq!(fan_speed(  8, &mut ring),   0);
        assert_eq!(fan_speed( 15, &mut ring),   1);
        assert_eq!(fan_speed( 25, &mut ring),   2);
        assert_eq!(fan_speed( 35, &mut ring),   5);
        assert_eq!(fan_speed( 45, &mut ring),  14);
        assert_eq!(fan_speed( 70, &mut ring),  65);
        assert_eq!(fan_speed( 80, &mut ring), 100);
    }

    #[test]
    fn test_ring_buffer() {
        let mut ring = HeapRb::new(3);
        // Come up with a set of values that will average
        // up and be able to prove the ring buffer is working.

        // Average will be 0, we'll expect the fan to be zero.
        assert_eq!(fan_speed(  0, &mut ring),   0);
        // Average will be 10, we'll expect the fan to be 1
        assert_eq!(fan_speed( 20, &mut ring),   1);
        // Average will be 20, we'll expect the fan to be 2
        assert_eq!(fan_speed( 40, &mut ring),   2);
        // The zero should fall out, and the average will be 30
        assert_eq!(fan_speed( 30, &mut ring),   3);
        // The 10 should fall out, and the average will be 30
        assert_eq!(fan_speed( 50, &mut ring),   8);
    }
}
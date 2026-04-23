pub fn sma(values: &[f64], period: usize) -> Option<f64> {
    if period == 0 || values.len() < period {
        return None;
    }

    let slice = &values[values.len() - period..];
    Some(slice.iter().sum::<f64>() / period as f64)
}

pub fn ema(values: &[f64], period: usize) -> Option<f64> {
    if period == 0 || values.len() < period {
        return None;
    }

    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut current = values[..period].iter().sum::<f64>() / period as f64;
    for value in &values[period..] {
        current = (*value - current) * multiplier + current;
    }

    Some(current)
}

pub fn rsi(values: &[f64], period: usize) -> Option<f64> {
    if period == 0 || values.len() <= period {
        return None;
    }

    let mut gains = 0.0;
    let mut losses = 0.0;
    for window in values.windows(2).rev().take(period) {
        let change = window[1] - window[0];
        if change >= 0.0 {
            gains += change;
        } else {
            losses += change.abs();
        }
    }

    if losses == 0.0 {
        return Some(100.0);
    }

    let rs = gains / losses;
    Some(100.0 - (100.0 / (1.0 + rs)))
}

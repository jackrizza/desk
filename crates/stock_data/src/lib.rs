use yfinance_rs::core::conversions::money_to_f64;
use yfinance_rs::{Interval, Range, Ticker, YfClient};

use models::raw::{
    IndicatorLine, IndicatorPoint, IndicatorResult, RawStockData, RawStockDataEntry,
    StockIndicatorsResponse,
};

use chrono::{NaiveDateTime, SecondsFormat};
use traquer::{momentum, smooth, trend, volatility, volume};

fn string_to_interval(interval: &str) -> Option<Interval> {
    match interval {
        "1m" => Some(Interval::I1m),
        "2m" => Some(Interval::I2m),
        "5m" => Some(Interval::I5m),
        "15m" => Some(Interval::I15m),
        "30m" => Some(Interval::I30m),
        "60m" => Some(Interval::I1h),
        "90m" => Some(Interval::I90m),
        "1h" => Some(Interval::I1h),
        "1d" => Some(Interval::D1),
        "5d" => Some(Interval::D5),
        "1wk" => Some(Interval::W1),
        "1mo" => Some(Interval::M1),
        _ => None,
    }
}

fn string_to_range(range: &str) -> Option<Range> {
    match range {
        "1d" => Some(Range::D1),
        "5d" => Some(Range::D5),
        "1mo" => Some(Range::M1),
        "3mo" => Some(Range::M3),
        "6mo" => Some(Range::M6),
        "1y" => Some(Range::Y1),
        "2y" => Some(Range::Y2),
        "5y" => Some(Range::Y5),
        "10y" => Some(Range::Y10),
        "ytd" => Some(Range::Ytd),
        "max" => Some(Range::Max),
        _ => None,
    }
}

pub fn normalize_yfinance_date_string(date: &str) -> Result<String, Box<dyn std::error::Error>> {
    let naive = NaiveDateTime::parse_from_str(
        date.strip_suffix(" UTC")
            .ok_or("date must end with ` UTC`")?,
        "%Y-%m-%d %H:%M:%S",
    )?;

    Ok(naive.and_utc().to_rfc3339_opts(SecondsFormat::Secs, true))
}

pub async fn get_stock_data(
    symbol: String,
    range: String,
    interval: String,
    prepost: bool,
) -> Result<RawStockData, Box<dyn std::error::Error>> {
    let client = YfClient::default();
    let ticker = Ticker::new(&client, &symbol);

    let r = string_to_range(&range).ok_or("Invalid range")?;
    let i = string_to_interval(&interval).ok_or("Invalid interval")?;

    let history = ticker.history(Some(r), Some(i), prepost).await?;

    let stock_data = history
        .iter()
        .map(|entry| {
            let date = match normalize_yfinance_date_string(&entry.ts.to_string()) {
                Ok(d) => d,
                Err(e) => format!("Invalid date: {}", e),
            };
            RawStockDataEntry {
                date,
                open: money_to_f64(&entry.open).to_string(),
                high: money_to_f64(&entry.high).to_string(),
                low: money_to_f64(&entry.low).to_string(),
                close: money_to_f64(&entry.close).to_string(),
                volume: match entry.volume {
                    Some(v) => v.to_string(),
                    // TODO: Consider using an Option<String> for volume in RawStockDataEntry to better represent missing data.
                    None => "No data".to_string(),
                },
            }
        })
        .collect();

    let last_refreshed = chrono::Utc::now().to_rfc3339();

    Ok(RawStockData {
        symbol,
        last_refreshed,
        interval,
        range,
        stock_data,
    })
}

const DEFAULT_WINDOW: usize = 14;

struct OhlcvSeries {
    dates: Vec<String>,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
}

fn series_from_raw(data: &RawStockData) -> OhlcvSeries {
    let dates = data
        .stock_data
        .iter()
        .map(|entry| entry.date.clone())
        .collect();
    let open = data
        .stock_data
        .iter()
        .map(|entry| entry.open.parse::<f64>().unwrap_or(f64::NAN))
        .collect();
    let high = data
        .stock_data
        .iter()
        .map(|entry| entry.high.parse::<f64>().unwrap_or(f64::NAN))
        .collect();
    let low = data
        .stock_data
        .iter()
        .map(|entry| entry.low.parse::<f64>().unwrap_or(f64::NAN))
        .collect();
    let close = data
        .stock_data
        .iter()
        .map(|entry| entry.close.parse::<f64>().unwrap_or(f64::NAN))
        .collect();
    let volume = data
        .stock_data
        .iter()
        .map(|entry| entry.volume.parse::<f64>().unwrap_or(0.0))
        .collect();

    OhlcvSeries {
        dates,
        open,
        high,
        low,
        close,
        volume,
    }
}

fn normalize_indicator_key(indicator: &str) -> String {
    indicator
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn line_from_values(dates: &[String], key: &str, label: &str, values: Vec<f64>) -> IndicatorLine {
    let points = values
        .into_iter()
        .zip(dates.iter())
        .filter_map(|(value, date)| {
            if value.is_finite() {
                Some(IndicatorPoint {
                    date: date.clone(),
                    value,
                })
            } else {
                None
            }
        })
        .collect();

    IndicatorLine {
        key: key.to_string(),
        label: label.to_string(),
        points,
    }
}

fn result_from_single(
    key: &str,
    display_name: &str,
    overlay: bool,
    dates: &[String],
    line_key: &str,
    line_label: &str,
    values: Vec<f64>,
) -> IndicatorResult {
    IndicatorResult {
        key: key.to_string(),
        display_name: display_name.to_string(),
        overlay,
        lines: vec![line_from_values(dates, line_key, line_label, values)],
    }
}

fn result_from_multi(
    key: &str,
    display_name: &str,
    overlay: bool,
    lines: Vec<IndicatorLine>,
) -> IndicatorResult {
    IndicatorResult {
        key: key.to_string(),
        display_name: display_name.to_string(),
        overlay,
        lines,
    }
}

fn map_tuple2(
    dates: &[String],
    series: impl Iterator<Item = (f64, f64)>,
    line_a: (&str, &str),
    line_b: (&str, &str),
) -> Vec<IndicatorLine> {
    let collected: Vec<(f64, f64)> = series.collect();
    let first = collected.iter().map(|(a, _)| *a).collect();
    let second = collected.iter().map(|(_, b)| *b).collect();

    vec![
        line_from_values(dates, line_a.0, line_a.1, first),
        line_from_values(dates, line_b.0, line_b.1, second),
    ]
}

fn map_tuple3(
    dates: &[String],
    series: impl Iterator<Item = (f64, f64, f64)>,
    line_a: (&str, &str),
    line_b: (&str, &str),
    line_c: (&str, &str),
) -> Vec<IndicatorLine> {
    let collected: Vec<(f64, f64, f64)> = series.collect();
    let first = collected.iter().map(|(a, _, _)| *a).collect();
    let second = collected.iter().map(|(_, b, _)| *b).collect();
    let third = collected.iter().map(|(_, _, c)| *c).collect();

    vec![
        line_from_values(dates, line_a.0, line_a.1, first),
        line_from_values(dates, line_b.0, line_b.1, second),
        line_from_values(dates, line_c.0, line_c.1, third),
    ]
}

fn percentile_bands(close: &[f64], bands: &[(f64, f64, f64)]) -> (Vec<f64>, Vec<f64>) {
    let mut percent_b = Vec::with_capacity(bands.len());
    let mut bandwidth = Vec::with_capacity(bands.len());

    for (index, (upper, middle, lower)) in bands.iter().enumerate() {
        let close_value = close.get(index).copied().unwrap_or(f64::NAN);
        let width = upper - lower;
        if !close_value.is_finite() || !upper.is_finite() || !lower.is_finite() || width == 0.0 {
            percent_b.push(f64::NAN);
            bandwidth.push(f64::NAN);
            continue;
        }

        percent_b.push((close_value - lower) / width);
        if middle.abs() < f64::EPSILON {
            bandwidth.push(f64::NAN);
        } else {
            bandwidth.push(width / middle);
        }
    }

    (percent_b, bandwidth)
}

fn channel_width(channels: &[(f64, f64, f64)]) -> Vec<f64> {
    channels
        .iter()
        .map(|(upper, _, lower)| {
            if upper.is_finite() && lower.is_finite() {
                upper - lower
            } else {
                f64::NAN
            }
        })
        .collect()
}

fn calculate_indicator(series: &OhlcvSeries, requested: &str) -> Option<IndicatorResult> {
    let key = normalize_indicator_key(requested);

    match key.as_str() {
        "adx_dms" => Some(result_from_multi(
            &key,
            requested,
            false,
            map_tuple3(
                &series.dates,
                trend::adx(
                    &series.high,
                    &series.low,
                    &series.close,
                    DEFAULT_WINDOW,
                    DEFAULT_WINDOW,
                ),
                ("adx", "ADX"),
                ("plus_di", "+DI"),
                ("minus_di", "-DI"),
            ),
        )),
        "atr_bands" => {
            let atr_values: Vec<f64> =
                volatility::atr(&series.high, &series.low, &series.close, DEFAULT_WINDOW).collect();
            let middle: Vec<f64> = smooth::ewma(&series.close, DEFAULT_WINDOW).collect();
            let upper: Vec<f64> = middle
                .iter()
                .zip(atr_values.iter())
                .map(|(mid, atr)| {
                    if mid.is_finite() && atr.is_finite() {
                        mid + (atr * 2.0)
                    } else {
                        f64::NAN
                    }
                })
                .collect();
            let lower: Vec<f64> = middle
                .iter()
                .zip(atr_values.iter())
                .map(|(mid, atr)| {
                    if mid.is_finite() && atr.is_finite() {
                        mid - (atr * 2.0)
                    } else {
                        f64::NAN
                    }
                })
                .collect();
            Some(result_from_multi(
                &key,
                requested,
                true,
                vec![
                    line_from_values(&series.dates, "upper", "Upper", upper),
                    line_from_values(&series.dates, "middle", "Middle", middle),
                    line_from_values(&series.dates, "lower", "Lower", lower),
                ],
            ))
        }
        "atr_trailing_stops" => Some(result_from_single(
            &key,
            requested,
            true,
            &series.dates,
            "atr_stop",
            "ATR Stop",
            trend::atr_stop(
                &series.high,
                &series.low,
                &series.close,
                DEFAULT_WINDOW,
                Some(3.0),
            )
            .collect(),
        )),
        "accumulation_distribution" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "ad",
            "A/D",
            volume::ad(
                &series.high,
                &series.low,
                &series.close,
                &series.volume,
                None,
            )
            .collect(),
        )),
        "accumulative_swing_index" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "asi",
            "ASI",
            trend::asi(&series.open, &series.high, &series.low, &series.close, 50.0).collect(),
        )),
        "alligator" => Some(result_from_multi(
            &key,
            requested,
            true,
            map_tuple3(
                &series.dates,
                trend::alligator(&series.close, 13, 8, 8, 5, 5, 3),
                ("jaw", "Jaw"),
                ("teeth", "Teeth"),
                ("lips", "Lips"),
            ),
        )),
        "aroon" => Some(result_from_multi(
            &key,
            requested,
            false,
            map_tuple2(
                &series.dates,
                trend::aroon(&series.high, &series.low, DEFAULT_WINDOW),
                ("up", "Aroon Up"),
                ("down", "Aroon Down"),
            ),
        )),
        "aroon_oscillator" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "aroon_oscillator",
            "Aroon Oscillator",
            trend::aroon(&series.high, &series.low, DEFAULT_WINDOW)
                .map(|(up, down)| {
                    if up.is_finite() && down.is_finite() {
                        up - down
                    } else {
                        f64::NAN
                    }
                })
                .collect(),
        )),
        "average_true_range" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "atr",
            "ATR",
            volatility::atr(&series.high, &series.low, &series.close, DEFAULT_WINDOW).collect(),
        )),
        "bollinger_bands" => Some({
            let bands: Vec<(f64, f64, f64)> =
                volatility::bbands(&series.close, 20, Some(2.0), Some(smooth::MaMode::EWMA))
                    .collect();
            result_from_multi(
                &key,
                requested,
                true,
                map_tuple3(
                    &series.dates,
                    bands.into_iter(),
                    ("upper", "Upper"),
                    ("middle", "Middle"),
                    ("lower", "Lower"),
                ),
            )
        }),
        "bollinger_b" => {
            let bands: Vec<(f64, f64, f64)> =
                volatility::bbands(&series.close, 20, Some(2.0), Some(smooth::MaMode::EWMA))
                    .collect();
            let (percent_b, _) = percentile_bands(&series.close, &bands);
            Some(result_from_single(
                &key,
                requested,
                false,
                &series.dates,
                "percent_b",
                "%B",
                percent_b,
            ))
        }
        "bollinger_bandwidth" => {
            let bands: Vec<(f64, f64, f64)> =
                volatility::bbands(&series.close, 20, Some(2.0), Some(smooth::MaMode::EWMA))
                    .collect();
            let (_, bandwidth) = percentile_bands(&series.close, &bands);
            Some(result_from_single(
                &key,
                requested,
                false,
                &series.dates,
                "bandwidth",
                "Bandwidth",
                bandwidth,
            ))
        }
        "chaikin_money_flow" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "cmf",
            "CMF",
            volume::cmf(&series.high, &series.low, &series.close, &series.volume, 20).collect(),
        )),
        "commodity_channel_index" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "cci",
            "CCI",
            momentum::cci(&series.high, &series.low, &series.close, 20).collect(),
        )),
        "donchian_channel" => Some(result_from_multi(
            &key,
            requested,
            true,
            map_tuple3(
                &series.dates,
                volatility::donchian(&series.high, &series.low, 20),
                ("upper", "Upper"),
                ("middle", "Middle"),
                ("lower", "Lower"),
            ),
        )),
        "donchian_width" => {
            let channels: Vec<(f64, f64, f64)> =
                volatility::donchian(&series.high, &series.low, 20).collect();
            Some(result_from_single(
                &key,
                requested,
                false,
                &series.dates,
                "width",
                "Width",
                channel_width(&channels),
            ))
        }
        "ease_of_movement" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "ease",
            "EOM",
            volume::ease(&series.high, &series.low, &series.volume, 14).collect(),
        )),
        "elder_force_index" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "elder_force",
            "EFI",
            volume::elder_force(&series.close, &series.volume, 13).collect(),
        )),
        "keltner_channel" => Some(result_from_multi(
            &key,
            requested,
            true,
            map_tuple3(
                &series.dates,
                volatility::keltner(&series.high, &series.low, &series.close, 20),
                ("middle", "Middle"),
                ("upper", "Upper"),
                ("lower", "Lower"),
            ),
        )),
        "klinger_volume_oscillator" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "kvo",
            "KVO",
            volume::kvo(
                &series.high,
                &series.low,
                &series.close,
                &series.volume,
                34,
                55,
                None,
            )
            .collect(),
        )),
        "macd" => {
            let macd: Vec<f64> = momentum::macd(&series.close, 12, 26).collect();
            let signal: Vec<f64> = smooth::ewma(&macd, 9).collect();
            let histogram: Vec<f64> = macd
                .iter()
                .zip(signal.iter())
                .map(|(macd_value, signal_value)| {
                    if macd_value.is_finite() && signal_value.is_finite() {
                        macd_value - signal_value
                    } else {
                        f64::NAN
                    }
                })
                .collect();
            Some(result_from_multi(
                &key,
                requested,
                false,
                vec![
                    line_from_values(&series.dates, "macd", "MACD", macd),
                    line_from_values(&series.dates, "signal", "Signal", signal),
                    line_from_values(&series.dates, "histogram", "Histogram", histogram),
                ],
            ))
        }
        "money_flow_index" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "mfi",
            "MFI",
            volume::mfi(&series.high, &series.low, &series.close, &series.volume, 14).collect(),
        )),
        "moving_average" => Some(result_from_single(
            &key,
            requested,
            true,
            &series.dates,
            "moving_average",
            "EMA (20)",
            smooth::ewma(&series.close, 20).collect(),
        )),
        "moving_average_envelope" => {
            let middle: Vec<f64> = smooth::ewma(&series.close, 20).collect();
            let upper = middle
                .iter()
                .map(|value| {
                    if value.is_finite() {
                        value * 1.02
                    } else {
                        f64::NAN
                    }
                })
                .collect();
            let lower = middle
                .iter()
                .map(|value| {
                    if value.is_finite() {
                        value * 0.98
                    } else {
                        f64::NAN
                    }
                })
                .collect();
            Some(result_from_multi(
                &key,
                requested,
                true,
                vec![
                    line_from_values(&series.dates, "upper", "Upper", upper),
                    line_from_values(&series.dates, "middle", "Middle", middle),
                    line_from_values(&series.dates, "lower", "Lower", lower),
                ],
            ))
        }
        "negative_volume_index" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "nvi",
            "NVI",
            volume::nvi(&series.close, &series.volume).collect(),
        )),
        "on_balance_volume" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "obv",
            "OBV",
            volume::obv(&series.close, &series.volume).collect(),
        )),
        "parabolic_sar" => Some(result_from_single(
            &key,
            requested,
            true,
            &series.dates,
            "psar",
            "PSAR",
            trend::psar(&series.high, &series.low, None, None).collect(),
        )),
        "positive_volume_index" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "pvi",
            "PVI",
            volume::pvi(&series.close, &series.volume).collect(),
        )),
        "price_oscillator" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "ppo",
            "PPO",
            momentum::ppo(&series.close, 12, 26).collect(),
        )),
        "price_rate_of_change" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "roc",
            "ROC",
            momentum::roc(&series.close, 12).collect(),
        )),
        "price_volume_trend" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "vpt",
            "PVT",
            volume::vpt(&series.close, &series.volume).collect(),
        )),
        "rsi" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "rsi",
            "RSI",
            momentum::rsi(&series.close, DEFAULT_WINDOW).collect(),
        )),
        "standard_deviation" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "std_dev",
            "Std Dev",
            volatility::std_dev(&series.close, 20, None).collect(),
        )),
        "stochastics" => Some(result_from_multi(
            &key,
            requested,
            false,
            map_tuple2(
                &series.dates,
                momentum::stochastic(&series.high, &series.low, &series.close, 14),
                ("k", "%K"),
                ("d", "%D"),
            ),
        )),
        "super_trend" => Some(result_from_single(
            &key,
            requested,
            true,
            &series.dates,
            "supertrend",
            "SuperTrend",
            trend::supertrend(&series.high, &series.low, &series.close, 10, 3.0).collect(),
        )),
        "trix" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "trix",
            "TRIX",
            momentum::trix(&series.close, 15).collect(),
        )),
        "true_range" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "true_range",
            "True Range",
            volatility::tr(&series.high, &series.low, &series.close).collect(),
        )),
        "twiggs_money_flow" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "twiggs",
            "Twiggs",
            volume::twiggs(&series.high, &series.low, &series.close, &series.volume, 21).collect(),
        )),
        "typical_price" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "typical_price",
            "Typical Price",
            volatility::typical(&series.high, &series.low, &series.close, 1).collect(),
        )),
        "ulcer_index" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "ulcer",
            "Ulcer Index",
            trend::ulcer(&series.close, 14).collect(),
        )),
        "ultimate_oscillator" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "ultimate",
            "Ultimate",
            momentum::ultimate(&series.high, &series.low, &series.close, 7, 14, 28).collect(),
        )),
        "vwap" => Some(result_from_single(
            &key,
            requested,
            true,
            &series.dates,
            "vwap",
            "VWAP",
            volume::vwap(
                &series.high,
                &series.low,
                &series.close,
                &series.volume,
                None,
            )
            .collect(),
        )),
        "vortex_index" => Some(result_from_multi(
            &key,
            requested,
            false,
            map_tuple2(
                &series.dates,
                trend::vortex(&series.high, &series.low, &series.close, 14),
                ("positive", "Positive"),
                ("negative", "Negative"),
            ),
        )),
        "williams_r" => Some(result_from_single(
            &key,
            requested,
            false,
            &series.dates,
            "williams_r",
            "Williams %R",
            momentum::wpr(&series.high, &series.low, &series.close, 14).collect(),
        )),
        "zigzag" => Some(result_from_single(
            &key,
            requested,
            true,
            &series.dates,
            "zigzag",
            "ZigZag",
            trend::zigzag(&series.high, &series.low, Some(5.0)).collect(),
        )),
        _ => None,
    }
}

pub fn calculate_indicators(data: &RawStockData, indicators: &[String]) -> StockIndicatorsResponse {
    let series = series_from_raw(data);
    let mut calculated = Vec::new();
    let mut unsupported = Vec::new();

    for requested in indicators {
        if let Some(result) = calculate_indicator(&series, requested) {
            calculated.push(result);
        } else {
            unsupported.push(requested.clone());
        }
    }

    StockIndicatorsResponse {
        symbol: data.symbol.clone(),
        last_refreshed: data.last_refreshed.clone(),
        interval: data.interval.clone(),
        range: data.range.clone(),
        indicators: calculated,
        unsupported,
    }
}

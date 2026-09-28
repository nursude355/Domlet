use wasm_bindgen::JsValue;
use web_sys::Element;

/// A small SVG line chart for telemetry-style browser UIs.
///
/// It has no animation loop, canvas runtime, or JavaScript dependency. Call
/// [`set_points`](Self::set_points) whenever the application state changes.
pub struct LineChart {
    svg: Element,
    line: Element,
}

impl LineChart {
    pub fn mount(host: &Element) -> Result<Self, JsValue> {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
        let namespace = Some("http://www.w3.org/2000/svg");
        let svg = document.create_element_ns(namespace, "svg")?;
        svg.set_attribute("viewBox", "0 0 100 100")?;
        svg.set_attribute("preserveAspectRatio", "none")?;
        svg.set_attribute("role", "img")?;
        svg.set_attribute("aria-label", "Live values")?;
        svg.set_attribute("width", "100%")?;
        svg.set_attribute("height", "100%")?;
        let line = document.create_element_ns(namespace, "polyline")?;
        line.set_attribute("fill", "none")?;
        line.set_attribute("stroke", "currentColor")?;
        line.set_attribute("stroke-width", "2")?;
        // The viewBox is stretched to the host, so keep the stroke width in
        // screen pixels instead of scaling it with the chart.
        line.set_attribute("vector-effect", "non-scaling-stroke")?;
        svg.append_child(&line)?;
        host.append_child(&svg)?;
        Ok(Self { svg, line })
    }

    /// Replaces the line with finite values, normalized to their own range.
    ///
    /// Proportionally scaled inputs therefore draw the same shape. Use
    /// [`set_points_in_range`](Self::set_points_in_range) when the vertical
    /// position must reflect absolute values.
    pub fn set_points(&self, values: &[f64]) -> Result<(), JsValue> {
        let points = finite(values);
        let low = points.iter().copied().fold(f64::INFINITY, f64::min);
        let high = points.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        self.draw(&points, low, high)
    }

    /// Replaces the line with finite values plotted on a fixed `low..=high`
    /// scale. Values outside the range are clamped to the chart edges.
    pub fn set_points_in_range(&self, values: &[f64], low: f64, high: f64) -> Result<(), JsValue> {
        if !(low.is_finite() && high.is_finite() && low < high) {
            return Err(JsValue::from_str(
                "chart range must be finite and non-empty",
            ));
        }
        self.draw(&finite(values), low, high)
    }

    fn draw(&self, points: &[f64], low: f64, high: f64) -> Result<(), JsValue> {
        if points.len() < 2 {
            return self.line.set_attribute("points", "");
        }
        self.line
            .set_attribute("points", &encode(points, low, high))
    }
}

impl Drop for LineChart {
    fn drop(&mut self) {
        self.svg.remove();
    }
}

fn finite(values: &[f64]) -> Vec<f64> {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect()
}

fn encode(points: &[f64], low: f64, high: f64) -> String {
    let span = (high - low).max(f64::EPSILON);
    let denominator = (points.len() - 1) as f64;
    points
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let height = ((value - low) * 100.0 / span).clamp(0.0, 100.0);
            format!(
                "{:.3},{:.3}",
                index as f64 * 100.0 / denominator,
                100.0 - height
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_range_reflects_absolute_values() {
        assert_eq!(
            encode(&[0.0, 50.0], 0.0, 100.0),
            "0.000,100.000 100.000,50.000"
        );
        assert_eq!(
            encode(&[0.0, 100.0], 0.0, 200.0),
            "0.000,100.000 100.000,50.000"
        );
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        assert_eq!(
            encode(&[-5.0, 500.0], 0.0, 100.0),
            "0.000,100.000 100.000,0.000"
        );
    }
}

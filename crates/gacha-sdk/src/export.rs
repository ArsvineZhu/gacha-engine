use crate::engine::{io_error, json_error};
use crate::*;
use std::{fs, path::Path};

impl StudyReport {
    pub fn write_json(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::write(path, serde_json::to_vec(self).map_err(json_error)?).map_err(io_error)
    }
    pub fn write_csv(&self, path: impl AsRef<Path>) -> Result<()> {
        let size = self.analysis.draws_per_action;
        let mut text = "draw,action,pmf,cdf,survival,sample_count,sample_cdf\n".to_string();
        for (p, s) in self
            .analysis
            .draw_distribution
            .distribution
            .iter()
            .zip(&self.simulation.distribution)
        {
            text.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                p.draw,
                p.draw.div_ceil(size),
                p.first_hit_probability,
                p.cumulative_probability,
                p.survival_probability,
                s.count,
                s.cumulative_probability
            ));
        }
        fs::write(path, text).map_err(io_error)
    }
}

//! Visual regression checking — SSIM computation and screenshot management.

#[cfg(test)]
use image::DynamicImage;
use std::path::{Path, PathBuf};

/// Compares screenshots against baselines for visual regression detection.
pub struct VisualRegressionChecker {
    baseline_dir: PathBuf,
    output_dir: PathBuf,
    ssim_threshold: f64,
}

impl VisualRegressionChecker {
    pub fn new(baseline_dir: impl AsRef<Path>, output_dir: impl AsRef<Path>) -> Self {
        let baseline = baseline_dir.as_ref().to_path_buf();
        let output = output_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&baseline).ok();
        std::fs::create_dir_all(&output).ok();
        Self {
            baseline_dir: baseline,
            output_dir: output,
            ssim_threshold: 0.95,
        }
    }

    /// Compute SSIM between two images. Returns value in [0.0, 1.0].
    pub fn compute_ssim(&self, img1_path: &Path, img2_path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
        let img1 = image::open(img1_path)?.to_luma8();
        let img2 = image::open(img2_path)?.to_luma8();

        // Resize to common dimensions
        let target_w = img1.width().min(img2.width());
        let target_h = img1.height().min(img2.height());
        let img1 = image::imageops::resize(&img1, target_w, target_h, image::imageops::FilterType::Lanczos3);
        let img2 = image::imageops::resize(&img2, target_w, target_h, image::imageops::FilterType::Lanczos3);

        let arr1: Vec<f64> = img1.pixels().map(|p| p[0] as f64).collect();
        let arr2: Vec<f64> = img2.pixels().map(|p| p[0] as f64).collect();

        let n = arr1.len() as f64;
        let mu1 = arr1.iter().sum::<f64>() / n;
        let mu2 = arr2.iter().sum::<f64>() / n;

        let sigma1 = (arr1.iter().map(|x| (x - mu1).powi(2)).sum::<f64>() / n).sqrt();
        let sigma2 = (arr2.iter().map(|x| (x - mu2).powi(2)).sum::<f64>() / n).sqrt();
        let sigma12 = arr1.iter().zip(&arr2).map(|(a, b)| (a - mu1) * (b - mu2)).sum::<f64>() / n;

        let k1 = 0.01_f64;
        let k2 = 0.03_f64;
        let l = 255.0_f64;
        let c1 = (k1 * l).powi(2);
        let c2 = (k2 * l).powi(2);

        let ssim = ((2.0 * mu1 * mu2 + c1) * (2.0 * sigma12 + c2))
            / ((mu1.powi(2) + mu2.powi(2) + c1) * (sigma1.powi(2) + sigma2.powi(2) + c2));

        Ok(ssim)
    }

    /// Compare a screenshot against its baseline.
    /// Returns (ssim, regression_detected).
    pub fn compare_against_baseline(
        &self,
        name: &str,
        screenshot_path: &Path,
    ) -> Result<(f64, bool), Box<dyn std::error::Error>> {
        let baseline_path = self.baseline_dir.join(format!("{}.png", name));

        if !baseline_path.exists() {
            // First run: save as baseline
            std::fs::copy(screenshot_path, &baseline_path)?;
            tracing::info!("Saved new baseline: {:?}", baseline_path);
            return Ok((1.0, false));
        }

        let ssim = self.compute_ssim(&baseline_path, screenshot_path)?;
        let regression = ssim < self.ssim_threshold;

        if regression {
            tracing::warn!(
                "Visual regression detected for '{}': SSIM={:.4} (threshold={:.2})",
                name, ssim, self.ssim_threshold
            );
        } else {
            tracing::info!("Visual check passed for '{}': SSIM={:.4}", name, ssim);
        }

        Ok((ssim, regression))
    }

    /// Update the baseline for a given test to the current screenshot.
    pub fn update_baseline(&self, name: &str, screenshot_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let baseline_path = self.baseline_dir.join(format!("{}.png", name));
        std::fs::copy(screenshot_path, &baseline_path)?;
        tracing::info!("Updated baseline: {:?}", baseline_path);
        Ok(())
    }

    /// Generate a timestamped screenshot path.
    pub fn screenshot_path(&self, label: &str) -> PathBuf {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        self.output_dir.join(format!("{}_{}.png", label, timestamp))
    }

    /// Simple pixel-difference fallback when SSIM is not available.
    pub fn pixel_diff_ratio(&self, img1_path: &Path, img2_path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
        let img1 = image::open(img1_path)?.to_rgb8();
        let img2 = image::open(img2_path)?.to_rgb8();

        let target_w = img1.width().min(img2.width());
        let target_h = img1.height().min(img2.height());
        let img1 = image::imageops::resize(&img1, target_w, target_h, image::imageops::FilterType::Nearest);
        let img2 = image::imageops::resize(&img2, target_w, target_h, image::imageops::FilterType::Nearest);

        let mut diff_pixels = 0u64;
        let total_pixels = (target_w * target_h) as u64;
        let threshold = 30u8;

        for (p1, p2) in img1.pixels().zip(img2.pixels()) {
            let r_diff = p1[0].abs_diff(p2[0]);
            let g_diff = p1[1].abs_diff(p2[1]);
            let b_diff = p1[2].abs_diff(p2[2]);
            if r_diff > threshold || g_diff > threshold || b_diff > threshold {
                diff_pixels += 1;
            }
        }

        let ratio = diff_pixels as f64 / total_pixels as f64;
        Ok(ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssim_identical_images() {
        let checker = VisualRegressionChecker::new("/tmp/test_baselines", "/tmp/test_output");
        // Create a simple test image
        let img = DynamicImage::new_rgb8(100, 100);
        let path1 = PathBuf::from("/tmp/test_img1.png");
        let path2 = PathBuf::from("/tmp/test_img2.png");
        img.save_with_format(&path1, ImageFormat::Png).unwrap();
        img.save_with_format(&path2, ImageFormat::Png).unwrap();

        let ssim = checker.compute_ssim(&path1, &path2).unwrap();
        assert!((ssim - 1.0).abs() < 0.01, "SSIM of identical images should be ~1.0, got {}", ssim);
    }
}

//! Safe Rust types mirroring the C SDK structs and enums.
//!
//! All types drop the `SVB_` prefix since the `svbony::` namespace provides it.
//! Enums implement [`TryFrom<c_int>`] for conversion from raw SDK values, and
//! structs implement [`From`] for conversion from their `svbony_sys` counterparts.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

/// Converts a null-terminated, fixed-size `c_char` buffer to an owned `String`.
fn chars_to_string(buf: &[c_char]) -> String {
    unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

macro_rules! c_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$vmeta:meta])* $variant:ident = $val:expr),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(i32)]
        $vis enum $name {
            $($(#[$vmeta])* $variant = $val),+
        }

        impl TryFrom<c_int> for $name {
            type Error = crate::Error;
            fn try_from(v: c_int) -> crate::Result<Self> {
                match v {
                    $($val => Ok(Self::$variant),)+
                    _ => Err(crate::Error::Unknown(v as i32)),
                }
            }
        }
    };
}

c_enum! {
    /// Output pixel format.
    ///
    /// The "Raw" variants deliver Bayer-mosaic data; the "Y" variants deliver
    /// debayered luminance. Formats wider than 8 bits are packed into 16-bit
    /// (little-endian) containers.
    #[derive(serde::Deserialize)]
    pub enum ImageType {
        Raw8 = 0,
        Raw10 = 1,
        Raw12 = 2,
        Raw14 = 3,
        Raw16 = 4,
        Y8 = 5,
        Y10 = 6,
        Y12 = 7,
        Y14 = 8,
        Y16 = 9,
        Rgb24 = 10,
        Rgb32 = 11,
    }
}

c_enum! {
    /// Bayer filter arrangement on a color sensor.
    pub enum BayerPattern {
        /// RGGB
        Rg = 0,
        /// BGGR
        Bg = 1,
        /// GRBG
        Gr = 2,
        /// GBRG
        Gb = 3,
    }
}

c_enum! {
    /// Adjustable camera control.
    pub enum ControlType {
        Gain = 0,
        Exposure = 1,
        Gamma = 2,
        GammaContrast = 3,
        WbR = 4,
        WbG = 5,
        WbB = 6,
        Flip = 7,
        /// 0 = low, 1 = medium, 2 = high.
        FrameSpeedMode = 8,
        Contrast = 9,
        Sharpness = 10,
        Saturation = 11,
        AutoTargetBrightness = 12,
        /// Black level (pedestal) offset.
        BlackLevel = 13,
        /// 0 = off, 1 = on.
        CoolerEnable = 14,
        /// In units of 0.1 C.
        TargetTemperature = 15,
        /// In units of 0.1 C (read-only).
        CurrentTemperature = 16,
        /// 0-100 % (read-only).
        CoolerPower = 17,
        BadPixelCorrEnable = 18,
        BadPixelCorrThreshold = 19,
    }
}

c_enum! {
    /// Camera capture / trigger mode.
    pub enum CameraMode {
        /// Free-running continuous capture.
        Normal = 0,
        /// Software trigger.
        TrigSoft = 1,
        /// Hardware trigger on rising edge.
        TrigRiseEdge = 2,
        /// Hardware trigger on falling edge.
        TrigFallEdge = 3,
        /// Hardware trigger on both edges.
        TrigDoubleEdge = 4,
        /// Hardware trigger while signal is high.
        TrigHighLevel = 5,
        /// Hardware trigger while signal is low.
        TrigLowLevel = 6,
    }
}

c_enum! {
    /// Image flip / mirror state.
    pub enum FlipStatus {
        None = 0,
        Horizontal = 1,
        Vertical = 2,
        Both = 3,
    }
}

c_enum! {
    /// ST4 pulse guide direction.
    pub enum GuideDirection {
        North = 0,
        South = 1,
        East = 2,
        West = 3,
    }
}

c_enum! {
    /// Trigger output pin selection.
    pub enum TrigOutputPin {
        PinA = 0,
        PinB = 1,
    }
}

c_enum! {
    /// Snapshot exposure status (used in trigger modes).
    pub enum ExposureStatus {
        /// Idle, ready to start.
        Idle = 0,
        /// Exposure in progress.
        Working = 1,
        /// Exposure finished, frame ready.
        Success = 2,
        /// Exposure failed.
        Failed = 3,
    }
}

impl ImageType {
    /// Returns the number of bytes per pixel for this image type.
    ///
    /// For packed formats (Raw10/12/14, Y10/12/14) the SDK delivers
    /// 2 bytes per pixel (16-bit little-endian container), same as Raw16/Y16.
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Raw8 | Self::Y8 => 1,
            Self::Raw10 | Self::Raw12 | Self::Raw14 | Self::Raw16 => 2,
            Self::Y10 | Self::Y12 | Self::Y14 | Self::Y16 => 2,
            Self::Rgb24 => 3,
            Self::Rgb32 => 4,
        }
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Basic camera identification, available without opening the camera.
///
/// Converted from [`svbony_sys::SVB_CAMERA_INFO`].
#[derive(Debug, Clone)]
pub struct CameraInfo {
    /// Human-readable camera name (e.g. "SV305 Pro").
    pub name: String,
    /// Camera serial number string.
    pub serial: String,
    /// Connection type (e.g. "USB3.0").
    pub port_type: String,
    /// OS-level device identifier.
    pub device_id: u32,
    /// SDK camera handle, passed to [`Camera::open`](crate::Camera::open).
    pub camera_id: i32,
}

impl From<&svbony_sys::SVB_CAMERA_INFO> for CameraInfo {
    fn from(c: &svbony_sys::SVB_CAMERA_INFO) -> Self {
        Self {
            name: chars_to_string(&c.FriendlyName),
            serial: chars_to_string(&c.CameraSN),
            port_type: chars_to_string(&c.PortType),
            device_id: c.DeviceID,
            camera_id: c.CameraID,
        }
    }
}

/// Static sensor properties (resolution, color, supported formats).
///
/// Converted from [`svbony_sys::SVB_CAMERA_PROPERTY`].
#[derive(Debug, Clone)]
pub struct CameraProperty {
    /// Maximum sensor width in pixels.
    pub max_width: i64,
    /// Maximum sensor height in pixels.
    pub max_height: i64,
    /// `true` if the sensor has a Bayer color filter.
    pub is_color: bool,
    /// Bayer pattern layout (only meaningful when `is_color` is `true`).
    pub bayer_pattern: BayerPattern,
    /// Supported binning factors (e.g. `[1, 2]`).
    pub supported_bins: Vec<i32>,
    /// Supported output pixel formats.
    pub supported_formats: Vec<ImageType>,
    /// Maximum ADC bit depth.
    pub max_bit_depth: i32,
    /// `true` if the camera supports trigger modes.
    pub is_trigger_cam: bool,
}

impl From<&svbony_sys::SVB_CAMERA_PROPERTY> for CameraProperty {
    fn from(c: &svbony_sys::SVB_CAMERA_PROPERTY) -> Self {
        let supported_bins = c
            .SupportedBins
            .iter()
            .copied()
            .take_while(|&b| b != 0)
            .collect();

        let supported_formats = c
            .SupportedVideoFormat
            .iter()
            .copied()
            .take_while(|&f| f != svbony_sys::SVB_IMG_END)
            .filter_map(|f| ImageType::try_from(f).ok())
            .collect();

        Self {
            max_width: c.MaxWidth as i64,
            max_height: c.MaxHeight as i64,
            is_color: c.IsColorCam != 0,
            bayer_pattern: BayerPattern::try_from(c.BayerPattern).unwrap_or(BayerPattern::Rg),
            supported_bins,
            supported_formats,
            max_bit_depth: c.MaxBitDepth,
            is_trigger_cam: c.IsTriggerCam != 0,
        }
    }
}

/// Extended camera properties.
///
/// Converted from [`svbony_sys::SVB_CAMERA_PROPERTY_EX`].
#[derive(Debug, Clone)]
pub struct CameraPropertyEx {
    /// `true` if the camera has an ST4 guide port.
    pub supports_pulse_guide: bool,
    /// `true` if the camera has a temperature sensor or TEC cooler.
    pub supports_temp_control: bool,
}

impl From<&svbony_sys::SVB_CAMERA_PROPERTY_EX> for CameraPropertyEx {
    fn from(c: &svbony_sys::SVB_CAMERA_PROPERTY_EX) -> Self {
        Self {
            supports_pulse_guide: c.bSupportPulseGuide != 0,
            supports_temp_control: c.bSupportControlTemp != 0,
        }
    }
}

/// Description of a single camera control's capabilities and range.
///
/// Converted from [`svbony_sys::SVB_CONTROL_CAPS`].
#[derive(Debug, Clone)]
pub struct ControlCaps {
    /// Control name (e.g. "Gain", "Exposure").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Maximum allowed value.
    pub max_value: i64,
    /// Minimum allowed value.
    pub min_value: i64,
    /// Factory default value.
    pub default_value: i64,
    /// `true` if the control supports auto mode.
    pub is_auto_supported: bool,
    /// `true` if the value can be written (some controls are read-only).
    pub is_writable: bool,
    /// Which control this describes.
    pub control_type: ControlType,
}

impl From<&svbony_sys::SVB_CONTROL_CAPS> for ControlCaps {
    fn from(c: &svbony_sys::SVB_CONTROL_CAPS) -> Self {
        Self {
            name: chars_to_string(&c.Name),
            description: chars_to_string(&c.Description),
            max_value: c.MaxValue as i64,
            min_value: c.MinValue as i64,
            default_value: c.DefaultValue as i64,
            is_auto_supported: c.IsAutoSupported != 0,
            is_writable: c.IsWritable != 0,
            control_type: ControlType::try_from(c.ControlType).unwrap_or(ControlType::Gain),
        }
    }
}

/// Region of interest (ROI) and binning settings.
#[derive(Debug, Clone, Copy)]
pub struct RoiFormat {
    /// Horizontal start offset in pixels.
    pub start_x: i32,
    /// Vertical start offset in pixels.
    pub start_y: i32,
    /// Width in pixels (post-binning). Must be a multiple of 8.
    pub width: i32,
    /// Height in pixels (post-binning). Must be a multiple of 2.
    pub height: i32,
    /// Binning factor (1 = no binning, 2 = 2x2, etc.).
    pub bin: i32,
}

/// Extended ROI settings including binning mode.
#[derive(Debug, Clone, Copy)]
pub struct RoiFormatEx {
    /// Horizontal start offset in pixels.
    pub start_x: i32,
    /// Vertical start offset in pixels.
    pub start_y: i32,
    /// Width in pixels (post-binning). Must be a multiple of 8.
    pub width: i32,
    /// Height in pixels (post-binning). Must be a multiple of 2.
    pub height: i32,
    /// Binning factor (1 = no binning, 2 = 2x2, etc.).
    pub bin: i32,
    /// Binning mode: 0 = average, 1 = sum.
    pub bin_mode: i32,
}

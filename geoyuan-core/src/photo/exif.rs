//! EXIF parsing module
//! 
//! Extracts GPS coordinates and metadata from image EXIF data

use crate::Result;
use std::io::Cursor;

/// EXIF data extracted from image
#[derive(Debug, Clone, Default)]
pub struct ExifData {
    pub gps: Option<super::GpsCoordinates>,
    pub timestamp: Option<i64>,
    pub make: Option<String>,
    pub model: Option<String>,
}

/// Parse EXIF from raw bytes
pub fn parse_exif(bytes: &[u8]) -> Result<ExifData> {
    parse_exif_from_slice(bytes)
}

/// Parse EXIF from slice (doesn't need file path)
pub fn parse_exif_from_slice(bytes: &[u8]) -> Result<ExifData> {
    let mut data = ExifData::default();
    
    // Try to parse EXIF using kamadak-exif
    let reader = exif::Reader::new();
    
    if let Ok(exif) = reader.read_from_container(&mut Cursor::new(bytes)) {
        // Extract GPS coordinates
        data.gps = extract_gps(&exif);
        
        // Extract timestamp
        data.timestamp = extract_timestamp(&exif);
        
        // Extract camera info
        data.make = exif.get_field(exif::Tag::Make, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string().trim().to_string());
            
        data.model = exif.get_field(exif::Tag::Model, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string().trim().to_string());
    }
    
    Ok(data)
}

/// Extract GPS coordinates from EXIF
fn extract_gps(exif: &exif::Exif) -> Option<super::GpsCoordinates> {
    let lat = extract_gps_coordinate(
        exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY)?,
        exif.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY)?
    )?;
    
    let lon = extract_gps_coordinate(
        exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY)?,
        exif.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY)?
    )?;
    
    let mut coords = super::GpsCoordinates::new(lat, lon);
    
    // Extract altitude if available
    if let Some(alt_field) = exif.get_field(exif::Tag::GPSAltitude, exif::In::PRIMARY) {
        if let exif::Value::Rational(ref rationals) = alt_field.value {
            if !rationals.is_empty() {
                coords.altitude = Some(rationals[0].num as f64 / rationals[0].denom as f64);
            }
        }
    }
    
    // Extract GPS timestamp if available
    if let Some(ts_field) = exif.get_field(exif::Tag::GPSTimeStamp, exif::In::PRIMARY) {
        if let exif::Value::Rational(ref rationals) = ts_field.value {
            if rationals.len() >= 3 {
                let hours = rationals[0].num as f64 / rationals[0].denom as f64;
                let minutes = rationals[1].num as f64 / rationals[1].denom as f64;
                let seconds = rationals[2].num as f64 / rationals[2].denom as f64;
                // Note: This is just time, not full date
                // Full date would need GPSDateStamp
                let _total_seconds = hours * 3600.0 + minutes * 60.0 + seconds;
            }
        }
    }
    
    Some(coords)
}

/// Extract a single GPS coordinate (latitude or longitude)
fn extract_gps_coordinate(
    coord_field: &exif::Field,
    ref_field: &exif::Field
) -> Option<f64> {
    let ref_char = ref_field.display_value().to_string().chars()
        .next()
        .unwrap_or('N');
    
    if let exif::Value::Rational(ref rationals) = coord_field.value {
        if rationals.len() >= 3 {
            let degrees = rationals[0].num as f64 / rationals[0].denom as f64;
            let minutes = rationals[1].num as f64 / rationals[1].denom as f64;
            let seconds = rationals[2].num as f64 / rationals[2].denom as f64;
            
            let mut decimal = degrees + minutes / 60.0 + seconds / 3600.0;
            
            // Apply reference (N/S or E/W)
            if ref_char == 'S' || ref_char == 'W' {
                decimal = -decimal;
            }
            
            return Some(decimal);
        }
    }
    
    None
}

/// Extract timestamp from EXIF
fn extract_timestamp(exif: &exif::Exif) -> Option<i64> {
    // Try DateTimeOriginal first
    let field = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        .or_else(|| exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY))?;
    
    // Format is typically "YYYY:MM:DD HH:MM:SS"
    let datetime_str = field.display_value().to_string();
    
    parse_exif_datetime(&datetime_str)
}

/// Parse EXIF datetime string to Unix timestamp
fn parse_exif_datetime(datetime_str: &str) -> Option<i64> {
    // Format: "YYYY:MM:DD HH:MM:SS"
    let parts: Vec<&str> = datetime_str.split(&[' ', ':'][..]).collect();
    
    if parts.len() >= 6 {
        let year: i32 = parts[0].parse().ok()?;
        let month: u32 = parts[1].parse().ok()?;
        let day: u32 = parts[2].parse().ok()?;
        let hour: u32 = parts[3].parse().ok()?;
        let minute: u32 = parts[4].parse().ok()?;
        let second: u32 = parts[5].parse().ok()?;
        
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(hour, minute, second))
            .map(|dt| dt.and_utc().timestamp())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_datetime() {
        let result = parse_exif_datetime("2024:03:15 14:30:00");
        assert!(result.is_some());
        let ts = result.unwrap();
        assert!(ts > 0);
    }
}

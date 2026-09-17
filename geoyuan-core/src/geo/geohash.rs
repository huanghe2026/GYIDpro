//! Geohash encoding module
//! 
//! Encodes GPS coordinates into geohash strings for efficient storage and map display

/// Base32 alphabet used by geohash
const BASE32: &[u8] = b"0123456789bcdefghjkmnpqrstuvwxyz";

/// Encode latitude and longitude into a geohash string
/// 
/// # Arguments
/// 
/// * `lat` - Latitude (-90 to 90)
/// * `lon` - Longitude (-180 to 180)
/// * `precision` - Number of characters in the hash (1-12)
///
/// # Example
/// 
/// ```no_run
/// // let hash = geoyuan_core::geo::geohash::encode(39.9042, 116.4074, 9);
/// // assert_eq!(hash, "wx4g0e6md5"); // Beijing
/// ```
pub fn encode(lat: f64, lon: f64, precision: usize) -> String {
    let precision = precision.min(12);
    
    let mut lat_range = (-90.0, 90.0);
    let mut lon_range = (-180.0, 180.0);
    
    let mut is_lon = true;
    let mut hash = Vec::with_capacity(precision);
    let mut bit = 0;
    let mut ch = 0u8;
    
    while hash.len() < precision {
        if is_lon {
            let mid = (lon_range.0 + lon_range.1) / 2.0;
            if lon >= mid {
                ch |= 1 << (4 - bit);
                lon_range.0 = mid;
            } else {
                lon_range.1 = mid;
            }
        } else {
            let mid = (lat_range.0 + lat_range.1) / 2.0;
            if lat >= mid {
                ch |= 1 << (4 - bit);
                lat_range.0 = mid;
            } else {
                lat_range.1 = mid;
            }
        }
        
        is_lon = !is_lon;
        bit += 1;
        
        if bit == 5 {
            hash.push(BASE32[ch as usize]);
            bit = 0;
            ch = 0;
        }
    }
    
    String::from_utf8(hash).unwrap()
}

/// Decode a geohash string back to latitude and longitude
/// Returns (latitude, longitude, latitude_error, longitude_error)
pub fn decode(hash: &str) -> Option<(f64, f64, f64, f64)> {
    let bytes = hash.as_bytes();
    if bytes.is_empty() || bytes.len() > 12 {
        return None;
    }
    
    let mut lat_range = (-90.0, 90.0);
    let mut lon_range = (-180.0, 180.0);
    let mut is_lon = true;
    
    for &byte in bytes {
        let idx = BASE32.iter().position(|&c| c == byte)?;
        
        for i in (0..5).rev() {
            let bit = (idx >> i) & 1;
            
            if is_lon {
                let mid = (lon_range.0 + lon_range.1) / 2.0;
                if bit == 1 {
                    lon_range.0 = mid;
                } else {
                    lon_range.1 = mid;
                }
            } else {
                let mid = (lat_range.0 + lat_range.1) / 2.0;
                if bit == 1 {
                    lat_range.0 = mid;
                } else {
                    lat_range.1 = mid;
                }
            }
            
            is_lon = !is_lon;
        }
    }
    
    let lat = (lat_range.0 + lat_range.1) / 2.0;
    let lon = (lon_range.0 + lon_range.1) / 2.0;
    let lat_err = (lat_range.1 - lat_range.0) / 2.0;
    let lon_err = (lon_range.1 - lon_range.0) / 2.0;
    
    Some((lat, lon, lat_err, lon_err))
}

/// Calculate bounding box for a geohash
pub fn bbox(hash: &str) -> Option<((f64, f64), (f64, f64))> {
    decode(hash).map(|(_, _, lat_err, lon_err)| {
        let (lat, lon, _, _) = decode(hash).unwrap();
        (
            (lat - lat_err, lon - lon_err), // SW corner
            (lat + lat_err, lon + lon_err), // NE corner
        )
    })
}

/// Get neighboring geohash
pub fn neighbor(hash: &str, direction: Direction) -> Option<String> {
    let (lat, lon, _, _) = decode(hash)?;
    
    let (dlat, dlon) = direction.offset();
    
    let new_lat = (lat + dlat).clamp(-90.0, 90.0);
    let new_lon = (lon + dlon).clamp(-180.0, 180.0);
    
    Some(encode(new_lat, new_lon, hash.len()))
}

/// Direction for neighbor lookup
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Northeast,
    Northwest,
    Southeast,
    Southwest,
}

impl Direction {
    fn offset(&self) -> (f64, f64) {
        // Approximate degrees per precision level
        let precision = 5; // Each character adds ~5 bits
        let lat_step = 180.0 / (1 << precision) as f64;
        let lon_step = 360.0 / (1 << precision) as f64;
        
        match self {
            Direction::North => (lat_step, 0.0),
            Direction::South => (-lat_step, 0.0),
            Direction::East => (0.0, lon_step),
            Direction::West => (0.0, -lon_step),
            Direction::Northeast => (lat_step, lon_step),
            Direction::Northwest => (lat_step, -lon_step),
            Direction::Southeast => (-lat_step, lon_step),
            Direction::Southwest => (-lat_step, -lon_step),
        }
    }
}

/// Get all 8 neighbors of a geohash
pub fn neighbors(hash: &str) -> Vec<(Direction, String)> {
    [
        Direction::North, Direction::South, Direction::East, Direction::West,
        Direction::Northeast, Direction::Northwest, Direction::Southeast, Direction::Southwest,
    ]
    .iter()
    .filter_map(|&dir| {
        neighbor(hash, dir).map(|h| (dir, h))
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encode_decode() {
        let lat = 39.9042;
        let lon = 116.4074;
        
        let hash = encode(lat, lon, 9);
        assert_eq!(hash.len(), 9);
        
        let (decoded_lat, decoded_lon, lat_err, lon_err) = decode(&hash).unwrap();
        
        assert!((lat - decoded_lat).abs() < lat_err * 2.0);
        assert!((lon - decoded_lon).abs() < lon_err * 2.0);
    }
    
    #[test]
    fn test_known_hashes() {
        // Known geohash for San Francisco (37.7749, -122.4194)
        let hash = encode(37.7749, -122.4194, 9);
        // Verify precision 9 produces 9 chars
        assert_eq!(hash.len(), 9);
        // Verify the hash starts with expected prefix for SF
        assert!(hash.starts_with("9q8yyk"), "Expected SF geohash to start with '9q8yyk', got: {}", hash);
        
        // Decode and verify within bounds
        let (lat, lon, _, _) = decode(&hash).unwrap();
        assert!((lat - 37.7749).abs() < 0.001);
        assert!((lon - (-122.4194)).abs() < 0.001);
    }
    
    #[test]
    fn test_precision() {
        let hash8 = encode(39.9042, 116.4074, 8);
        let hash9 = encode(39.9042, 116.4074, 9);
        let hash10 = encode(39.9042, 116.4074, 10);
        
        assert!(hash8.starts_with(&hash9[..8]));
        assert!(hash9.starts_with(&hash10[..9]));
    }
}

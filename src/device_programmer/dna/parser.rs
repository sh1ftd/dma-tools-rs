use crate::device_programmer::DnaInfo;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaParseError {
    InformationNotFound,
}

impl fmt::Display for DnaParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InformationNotFound => formatter.write_str("DNA information not found"),
        }
    }
}

pub fn parse_dna_output(contents: &str) -> Result<DnaInfo, DnaParseError> {
    let device_type = if contents.contains("CH347 Open Succ") {
        "CH347"
    } else if contents.contains("ftdi:") {
        "FTDI"
    } else {
        "Unknown"
    };

    let dna_line = contents
        .lines()
        .find(|line| line.trim().starts_with("DNA ="))
        .ok_or(DnaParseError::InformationNotFound)?;
    let value_part = dna_line
        .split_once('=')
        .map(|(_, value)| value.trim())
        .ok_or(DnaParseError::InformationNotFound)?;
    let (binary_part, hexadecimal_part) = value_part
        .split_once('(')
        .and_then(|(binary, hexadecimal)| {
            hexadecimal
                .find(')')
                .map(|end| (binary.trim(), hexadecimal[..end].trim()))
        })
        .ok_or(DnaParseError::InformationNotFound)?;

    let binary_value =
        u64::from_str_radix(binary_part, 2).map_err(|_| DnaParseError::InformationNotFound)?;
    let hexadecimal_digits = hexadecimal_part
        .strip_prefix("0x")
        .ok_or(DnaParseError::InformationNotFound)?;
    let hexadecimal_value = u64::from_str_radix(hexadecimal_digits, 16)
        .map_err(|_| DnaParseError::InformationNotFound)?;

    if binary_value != hexadecimal_value {
        return Err(DnaParseError::InformationNotFound);
    }

    Ok(DnaInfo {
        dna_value: hexadecimal_part.to_string(),
        dna_raw_value: binary_part.to_string(),
        device_type: device_type.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ch347_dna_output() {
        let contents = "\
Open On-Chip Debugger 0.12.0-rc3 (2024-01-26)
CH347 Open Succ
DNA = 001100100000111001100001001101010111010010110100001010100 (0x00641CC26AE96854)
";
        let info = parse_dna_output(contents).unwrap();
        assert_eq!(info.dna_value, "0x00641CC26AE96854");
        assert_eq!(
            info.dna_raw_value,
            "001100100000111001100001001101010111010010110100001010100"
        );
        assert_eq!(info.device_type, "CH347");
    }

    #[test]
    fn parses_ftdi_dna_output() {
        let contents = "Open On-Chip Debugger\nInfo : ftdi: initialized\nDNA = 1101 (0xD)\n";
        let info = parse_dna_output(contents).unwrap();
        assert_eq!(info.device_type, "FTDI");
        assert_eq!(info.dna_value, "0xD");
    }

    #[test]
    fn detects_unknown_device() {
        let info = parse_dna_output("DNA = 10101011 (0xAB)\n").unwrap();
        assert_eq!(info.device_type, "Unknown");
    }

    #[test]
    fn accepts_equivalent_values_with_leading_zeroes() {
        let info = parse_dna_output("DNA = 000000001101 (0x000D)\n").unwrap();

        assert_eq!(info.dna_raw_value, "000000001101");
        assert_eq!(info.dna_value, "0x000D");
    }

    #[test]
    fn rejects_mismatched_or_overflowing_values() {
        for contents in [
            "DNA = 0011 (0x4)\n",
            "DNA = 11111111111111111111111111111111111111111111111111111111111111111 (0x1)\n",
            "DNA = 1 (0x10000000000000000)\n",
        ] {
            assert_eq!(
                parse_dna_output(contents),
                Err(DnaParseError::InformationNotFound),
                "unexpectedly accepted {contents:?}"
            );
        }
    }

    #[test]
    fn rejects_missing_or_malformed_dna() {
        for contents in [
            "",
            "Open On-Chip Debugger\nDone.\n",
            "DNA = 001122INVALID (0xABC)\n",
            "DNA = 0011 (NOTHEX)\n",
            "DNA = 001100100000\n",
        ] {
            assert_eq!(
                parse_dna_output(contents),
                Err(DnaParseError::InformationNotFound),
                "unexpectedly accepted {contents:?}"
            );
        }
    }
}

use super::PcileechTestState;

const SUCCESS_SIGNATURE: &str = "ntdll.dll base address:";

pub fn find_success_line(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim();
        if is_error_line(line) {
            return None;
        }

        let (_, address) = line.split_once(SUCCESS_SIGNATURE)?;
        let address = address.trim();
        let hexadecimal = address
            .strip_prefix("0x")
            .or_else(|| address.strip_prefix("0X"))?;

        u64::from_str_radix(hexadecimal, 16)
            .ok()
            .filter(|address| *address != 0)
            .map(|_| line.to_string())
    })
}

pub fn find_error_message(output: &str) -> Option<String> {
    output
        .lines()
        .find(|line| line.contains("Error:"))
        .or_else(|| output.lines().find(|line| is_error_line(line)))
        .map(|line| line.trim().to_string())
}

fn is_error_line(line: &str) -> bool {
    line.to_ascii_lowercase().contains("error")
}

pub fn finalize_result(
    output: &str,
    success_line: Option<String>,
    process_error: Option<String>,
) -> PcileechTestState {
    if let Some(line) = success_line.or_else(|| find_success_line(output)) {
        return PcileechTestState::Success(line);
    }

    PcileechTestState::Failed(
        process_error
            .or_else(|| find_error_message(output))
            .unwrap_or_else(|| "Unknown error".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pcileech_success_signature() {
        let output = "memflow init\nntdll.dll base address: 0x7ffa0000\n";
        assert_eq!(
            find_success_line(output),
            Some("ntdll.dll base address: 0x7ffa0000".to_string())
        );
    }

    #[test]
    fn extracts_first_error_line() {
        let output = "startup\nError: failed to initialize connector\nmore detail\n";
        assert_eq!(
            find_error_message(output),
            Some("Error: failed to initialize connector".to_string())
        );
    }

    #[test]
    fn detects_lowercase_error_line() {
        let output = "connector error: device not found\n";
        assert_eq!(
            find_error_message(output),
            Some("connector error: device not found".to_string())
        );
    }

    #[test]
    fn valid_success_wins_over_prior_error() {
        let output = "Error: transient connector warning\nntdll.dll base address: 0x7ffa0000\n";
        assert_eq!(
            finalize_result(
                output,
                None,
                Some("Error: transient connector warning".to_string())
            ),
            PcileechTestState::Success("ntdll.dll base address: 0x7ffa0000".into())
        );
    }

    #[test]
    fn rejects_success_signature_on_error_line() {
        let output = "Error: missing ntdll.dll base address: 0x7ffa0000\n";

        assert_eq!(find_success_line(output), None);
        assert_eq!(
            finalize_result(output, None, None),
            PcileechTestState::Failed(output.trim().into())
        );
    }

    #[test]
    fn rejects_malformed_success_addresses() {
        for output in [
            "ntdll.dll base address:\n",
            "ntdll.dll base address: unknown\n",
            "ntdll.dll base address: 7ffa0000\n",
            "ntdll.dll base address: 0x0\n",
            "ntdll.dll base address: 0x7ffa0000 trailing\n",
        ] {
            assert_eq!(find_success_line(output), None, "accepted {output:?}");
        }
    }

    #[test]
    fn accepts_valid_success_with_a_benign_prefix() {
        let output = "status: ntdll.dll base address: 0x7ffa0000\n";

        assert_eq!(find_success_line(output), Some(output.trim().to_string()));
    }

    #[test]
    fn preserves_process_error_without_success() {
        assert_eq!(
            finalize_result("startup\n", None, Some("Error: failed".to_string())),
            PcileechTestState::Failed("Error: failed".into())
        );
    }
}

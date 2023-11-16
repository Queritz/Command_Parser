#![no_std]

use core::panic::PanicInfo;
use core::slice::from_raw_parts;
use nom::{bytes::complete::tag, error::Error, sequence::separated_pair, IResult, combinator::fail};

/// Searches a slice in a slice. If the needle is found in the haystack, the position of the first
/// matching byte is returned. If no needle is found, None is returned.
/// Somewhat dubious because .windows can panic if the length of need is 0.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// State of an Led.
#[derive(Debug, PartialEq)]
#[repr(C)]
pub enum LedState {
    On,
    Off,
}

impl LedState {
    /// Nom filter function. Checks if the slice contain a LedState.
    /// # To know:
    /// This function does not check if the match is clean. This means it
    /// detects things like: "on" and "off" but also "oonn" or "offasdf"
    ///                                                ^^       ^^^
    /// If a state is detected, the input gets split up after the state sequence.
    /// # Example
    /// in:                 out:
    /// input = "on"        Ok("LedState:On", ())
    /// input = "onnnnn"    Ok("LedState:On", "nnnn")
    /// input = "asdf"      Err("asdf")
    fn from_slice(input: &[u8]) -> IResult<&[u8], LedState> {
        const ON: &[u8] = b"on";
        const OFF: &[u8] = b"off";

        if let Some(pos) = find_subsequence(input, ON) {
            return Ok((&input[(pos + ON.len())..], LedState::On));
        }
        if let Some(pos) = find_subsequence(input, OFF) {
            return Ok((&input[(pos + OFF.len())..], LedState::Off));
        }
        fail(input)
    }
}

/// Represents the four led's on the board.
#[derive(Debug, PartialEq)]
#[repr(C)]
pub enum Led {
    Led1,
    Led2,
    Led3,
    Led4,
}

impl Led {
    /// Nom filter function. Checks if the slice contains any led.
    /// # To know:
    /// This function does not check if the match is clean. This means it
    /// detects things like: "led1" and "led2" but also "lled11" or "led2asdf"
    ///                                                   ^^^        ^^^
    /// If a state is detected, the input gets split up after the state sequence.
    /// # Example
    /// in:                 out:
    /// input = "led1"      Ok(Led::Led1, ())
    /// input = "asled2df"  Ok(Led::Led2, "df")
    /// input = "asdf"      Err("asdf")
    fn from_slice(input: &[u8]) -> IResult<&[u8], Led> {
        const LED1: &[u8] = b"led1";
        const LED2: &[u8] = b"led2";
        const LED3: &[u8] = b"led3";
        const LED4: &[u8] = b"led4";

        if let Some(pos) = find_subsequence(input, LED1) {
            return Ok((&input[(pos + LED1.len())..], Led::Led1));
        }
        if let Some(pos) = find_subsequence(input, LED2) {
            return Ok((&input[(pos + LED2.len())..], Led::Led2));
        }
        if let Some(pos) = find_subsequence(input, LED3) {
            return Ok((&input[(pos + LED3.len())..], Led::Led3));
        }
        if let Some(pos) = find_subsequence(input, LED4) {
            return Ok((&input[(pos + LED4.len())..], Led::Led4));
        }
        fail(input)
    }
}

/// Parsed command info, returned to the C code.
#[derive(Debug, PartialEq)]
#[repr(C)]
pub struct Command {
    /// Indicates if the parsing was successful. "Option" / "Result" is not FFI friendly.
    pub success: bool,
    /// Which led to operate on.
    pub led: Led,
    /// Which state to put the led in.
    pub state: LedState,
}

impl Command {
    /// Generate a command from a byte slice.
    fn from_slice(input: &[u8]) -> Self {
        // Per default, the parsing fails.
        let mut command = Command {
            success: false,
            led: Led::Led1,
            state: LedState::Off,
        };
        const ESP: &[u8] = b"esp ";
        const SPACE: &[u8] = b" ";

        // Check if the command starts with the keyword "esp"
        if let Ok((input, _)) = tag::<&[u8], &[u8], Error<_>>(ESP)(input) {
            // Extract the LED and state.
            if let Ok((_input, (led, state))) = separated_pair(Led::from_slice, tag(SPACE), LedState::from_slice)(input) {
                command.state = state;
                command.led = led;
                command.success = true;
            };
        };
        command
    }
}

/// Unsafe function that converts a pointer of bytes into a byte slice.
/// Needed because slices are not FFI friendly. Potentially dangerous if
/// a wrong length is give, but that must be handled on the C side.
fn bytes_to_slice(input: *const u8, length: u32) -> &'static [u8] {
    unsafe { from_raw_parts(input, length as usize) }
}

/// C FFI. Converts the ASCII stream into a usable command.
#[no_mangle]
pub extern "C" fn parse_uart(input: *const u8, length: u32) -> Command {
    Command::from_slice(bytes_to_slice(input, length))
}

/// Not sure how to handle a panic.
#[cfg_attr(not(test), panic_handler)]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_led1_on() {
        assert_eq!(
            parse_uart("esp led1 on".as_ptr(), "esp led1 on".len() as u32),
            Command {
                success: true,
                led: Led::Led1,
                state: LedState::On,
            }
        );
    }

    #[test]
    fn test_led2_off() {
        assert_eq!(
            parse_uart("esp led2 off".as_ptr(), "esp led2 off".len() as u32),
            Command {
                success: true,
                led: Led::Led2,
                state: LedState::Off,
            }
        );
    }

    #[test]
    fn test_led2_off_fail() {
        assert_eq!(
            parse_uart("esp led2 ofna".as_ptr(), "esp led2 ofna".len() as u32),
            Command {
                success: false,
                led: Led::Led1,
                state: LedState::Off,
            }
        );
    }

    #[test]
    fn test_led3_on_oversized() {
        assert_eq!(
            parse_uart("esp led3 on".as_ptr(), ("esp led3 on".len() + 2) as u32),
            Command {
                success: true,
                led: Led::Led3,
                state: LedState::On,
            }
        );
    }
}

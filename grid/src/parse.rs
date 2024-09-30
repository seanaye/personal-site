use std::str::FromStr;

use nom::{bytes::complete::tag, character::complete::u32, sequence::separated_pair, IResult};

use crate::{AspectRatio, Dimension};

fn num(s: &str) -> IResult<&str, u32> {
    u32(s)
}

impl FromStr for Dimension {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let res = separated_pair(num, tag("x"), num)(s);

        match res {
            Ok(("", (width, height))) => Ok(Dimension {
                width: width as usize,
                height: height as usize,
            }),
            Ok((x, _)) => Err(anyhow::anyhow!("leftover content: {}", x)),
            Err(x) => Err(anyhow::anyhow!("failed to parse: {}", x.to_string())),
        }
    }
}

impl FromStr for AspectRatio {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let res = separated_pair(num, tag(":"), num)(s);

        match res {
            Ok(("", (width, height))) => Ok(AspectRatio {
                width: width as usize,
                height: height as usize,
            }),
            Ok((x, _)) => Err(anyhow::anyhow!("leftover content: {}", x)),
            Err(x) => Err(anyhow::anyhow!("failed to parse: {}", x.to_string())),
        }
    }
}

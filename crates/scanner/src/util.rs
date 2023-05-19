use broadsword_address::Offset;

pub(crate) fn split_scannable(
    scannable: &'static [u8],
    chunks: usize,
    overlap: usize,
) -> Vec<(Offset, &'static [u8])> {
    let mut results = Vec::new();

    let bytes_per_chunk = scannable.len() / chunks;
    let mut offset = Offset::from(0);

    for _ in 0..chunks {
        let start: usize = offset.into();
        let end = clamp(
            start + (bytes_per_chunk + overlap),
            0,
            scannable.len()
        );

        results.push((offset.clone(), &scannable[start..end]));
        offset.move_by(bytes_per_chunk);
    }

    results
}

fn clamp(input: usize, min: usize, max: usize) -> usize {
    if input < min {
        return min;
    }

    if input > max {
        return max;
    }

    input
}

#[cfg(test)]
mod tests {}

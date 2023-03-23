// TODO: make zero-copy impl
pub(crate) fn split_scannable(
    scannable: &[u8],
    chunks: usize,
    overlap: usize
) -> Vec<(usize, Vec<u8>)> {
    let mut results = Vec::new();

    let bytes_per_chunk = scannable.len() / chunks;
    let mut current_offset = 0;

    for _ in 0..chunks {
        let start = current_offset;
        let end = clamp(start + (bytes_per_chunk + overlap), 0, scannable.len());

        results.push((start, scannable[start..end].to_vec()));
        current_offset = start + bytes_per_chunk;
    }

    results
}

fn clamp(input: usize, min: usize, max: usize) -> usize {
    if input < min {
        return min
    }

    if input > max {
        return max
    }

    input
}

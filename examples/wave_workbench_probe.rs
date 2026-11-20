use ondas::{TimeRange, ValueRef};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or("expected waveform path")?;
    let mut wave = ondas::open(path)?;
    let variables: Vec<_> = wave
        .hierarchy()
        .variables()
        .filter_map(|variable| Some((variable.path(), variable.signal()?)))
        .collect();
    for (path, signal) in variables {
        let trace = wave.trace(signal, TimeRange::all())?;
        for change in trace.changes() {
            if let ValueRef::Real(value) = change.value() {
                println!("{path} {} {value}", change.time().ticks());
            }
        }
    }
    Ok(())
}

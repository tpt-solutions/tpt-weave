use tpt_weave_cli::run;

fn main() {
    let outcome: tpt_weave_cli::RunOutcome = run(std::env::args().skip(1).collect());
    if !outcome.stdout.is_empty() {
        print!("{}", outcome.stdout);
    }
    if !outcome.stderr.is_empty() {
        eprint!("{}", outcome.stderr);
    }
    std::process::exit(outcome.code);
}

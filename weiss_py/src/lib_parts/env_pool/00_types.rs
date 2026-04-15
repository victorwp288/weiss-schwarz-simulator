#[pyclass(name = "EnvPool")]
struct PyEnvPool {
    pool: EnvPool,
}

enum RlPoolCtor {
    Train,
    Eval,
}

struct ParsedPoolInit {
    db: Arc<CardDb>,
    config: EnvConfig,
    curriculum: CurriculumConfig,
    error_policy: ErrorPolicy,
    visibility: ObservationVisibility,
    num_threads: Option<usize>,
    debug: DebugConfig,
}

impl PyEnvPool {}


    fn render_ansi(&self, env_index: usize, perspective: u8) -> PyResult<String> {
        let num_envs = self.pool.envs.len();
        if env_index >= num_envs {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "env_index {env_index} out of bounds (num_envs = {num_envs})"
            )));
        }
        Ok(self.pool.render_ansi(env_index, perspective))
    }

    #[getter]
    fn envs_len(&self) -> usize {
        self.pool.envs.len()
    }

    #[getter]
    fn num_envs(&self) -> usize {
        self.pool.envs.len()
    }

    #[getter]
    fn obs_len(&self) -> usize {
        OBS_LEN
    }

    #[getter]
    fn action_space(&self) -> usize {
        ACTION_SPACE_SIZE
    }

    #[getter]
    fn num_threads(&self) -> usize {
        self.pool.effective_num_threads()
    }

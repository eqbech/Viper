//! lib.rs

#[pyo3::pymodule]
mod viper {
    use indicatif::{ProgressBar, ProgressStyle};
    use log::{info, warn};
    use pyo3::prelude::*;
    use reqwest::Client;
    use serde_json::Value;
    use serde_query::{Deserialize, DeserializeQuery};
    use std::{
        collections::HashMap,
        sync::{Arc, LazyLock, Once},
        time::Duration,
    };
    use tokio::{
        runtime::Handle,
        sync::{Mutex, Semaphore},
        task::JoinSet,
    };

    static LOGGER_INIT: Once = Once::new();

    #[derive(Debug, Deserialize, DeserializeQuery)]
    #[pyclass]
    struct PyPiPackage {
        #[pyo3(get)]
        #[query(".info.name")]
        name: String,

        #[query(".releases")]
        _releases: HashMap<String, Value>,
    }

    enum ViperError {
        RequestError(reqwest::Error),
        ParseError(serde_json::Error),
    }

    impl From<ViperError> for PyErr {
        fn from(error: ViperError) -> Self {
            match error {
                ViperError::RequestError(e) => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Request error: {}", e),
                ),
                ViperError::ParseError(e) => {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Parse error: {}", e))
                }
            }
        }
    }

    #[pyclass]
    struct PyPiClient {
        internal_client: LazyLock<Client>,
    }

    #[pymethods]
    impl PyPiClient {
        #[new]
        fn new() -> Self {
            init_logger();
            Self {
                internal_client: LazyLock::new(|| {
                    Client::builder().hickory_dns(true).build().unwrap()
                }),
            }
        }

        #[pyo3(name = "get")]
        fn get<'a>(&'a self, py: Python<'a>, package: String) -> PyResult<Bound<'a, PyAny>> {
            let client_clone = self.internal_client.clone();

            pyo3_async_runtimes::tokio::future_into_py(py, async move {
                match request_package(client_clone, &package).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(PyErr::from(e)),
                }
            })
        }
        /// ## Core Logic
        /// This method uses tokio to spawn concurrent tasks for fetching multiple packages.
        /// The `max_concurrency` parameter allows you to limit the number of concurrent requests using a `semaphore`.
        ///
        /// ## Performance considerations
        /// The use of `semaphore` as a "rate limiter" is justified by the fact that PyPi does not seem to have any strict rate limiting if any at all.
        /// The reason for a max concurrency at all is to avoid overwhelming the local system.
        /// For more information about PyPi rate limiting, visit the [PyPi API documentation](https://docs.pypi.org/api/)
        #[pyo3(name = "get_many")]
        #[pyo3(signature = (packages, max_concurrency=None))]
        fn get_many<'a>(
            &'a self,
            py: Python<'a>,
            packages: Vec<String>,
            max_concurrency: Option<u32>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client_clone = self.internal_client.clone();

            pyo3_async_runtimes::tokio::future_into_py(py, async move {
                info!(
                    "Initialized tokio runtime with {} worker threads",
                    Handle::current().metrics().num_workers()
                );

                let num_concurrency: usize = match max_concurrency {
                    Some(c) => c as usize,
                    None => {
                        info!("No max concurrency specified, defaulting to 250");
                        250
                    }
                };
                let semaphore = Arc::new(Semaphore::new(num_concurrency));
                let successful_packages = Arc::new(Mutex::new(Vec::with_capacity(packages.len())));
                let failed_packages = Arc::new(Mutex::new(Vec::with_capacity(packages.len())));

                info!("Fetching {} PyPi packages", packages.len());
                let pb = Arc::new(Mutex::new(build_progress_bar(packages.len() as u64)));

                let handles = packages
                    .into_iter()
                    .map(|p| {
                        let (semaphore_clone, sc, fc, pbc, cc) = (
                            semaphore.clone(),
                            successful_packages.clone(),
                            failed_packages.clone(),
                            pb.clone(),
                            client_clone.clone(),
                        );
                        tokio::spawn(async move {
                            let _permit = semaphore_clone.acquire().await.unwrap();
                            match request_package(cc, &p).await {
                                Ok(pkg) => sc.lock().await.push(pkg),
                                Err(_) => fc.lock().await.push(p),
                            }
                            drop(_permit);
                            let pb = pbc.lock().await;
                            pb.inc(1);
                            let pos = pb.position();
                            let len = pb.length().unwrap_or(0);
                            pb.set_message(format!(
                                "[{}/{}] fetching packages...",
                                format_download(pos),
                                format_download(len)
                            ));
                        })
                    })
                    .collect::<JoinSet<_>>();

                handles.join_all().await;
                pb.lock().await.finish_with_message("Fetching complete!");

                let (s, f) = (
                    Arc::try_unwrap(successful_packages)
                        .expect("Lock still has multiple owners.")
                        .into_inner(),
                    Arc::try_unwrap(failed_packages)
                        .expect("Lock still has multiple owners.")
                        .into_inner(),
                );

                info!("Successfully fetched {} PyPi packages", s.len());
                if !f.is_empty() {
                    warn!("Failed to fetch {} PyPi packages", f.len());
                }

                Ok(s)
            })
        }
    }
    /// ## Core logic
    /// This functions performs the https request to PyPi and parses the response.
    /// it uses [serde_json](https://docs.serde.rs/serde_json/) to parse the JSON response into a `PyPiPackage` struct.
    ///
    /// ## Performance considerations
    /// This method is not optimal as it reads the entire response into memory before parsing it.
    /// Future improvements could include streaming the response and parsing only what you need and exit early when possible.
    /// However, the biggest improvement would be to use a different endpoint as this endpoints returns a lot of unecessary data
    /// that is not needed. I could not seem to find any endpoint satisfying this requirement, so this is what we have for now.
    async fn request_package(client: Client, package: &str) -> Result<PyPiPackage, ViperError> {
        match client
            .get(format!("https://pypi.org/pypi/{package}/json"))
            .send()
            .await
        {
            Ok(response) => {
                match serde_json::from_slice::<PyPiPackage>(&response.bytes().await.unwrap()) {
                    Ok(pypi) => Ok(pypi),
                    Err(e) => {
                        warn!("Failed to parse JSON for package {:?}", package);
                        Err(ViperError::ParseError(e))
                    }
                }
            }
            Err(e) => {
                warn!("Failed to fetch package {:?}: {}", package, e);
                Err(ViperError::RequestError(e))
            }
        }
    }

    // <-- Miscallaneous tools for logging and progress bar formatting -->

    fn init_logger() {
        LOGGER_INIT.call_once(|| {
            let mut builder =
                env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));

            builder
                .format_timestamp_secs()
                .format_module_path(false)
                .format_target(false)
                .format(|buf, record| {
                    use std::io::Write;

                    let (level, color) = match record.level() {
                        log::Level::Error => ("ERROR", "\x1b[31m"), // red
                        log::Level::Warn => ("WARN", "\x1b[33m"),   // yellow
                        log::Level::Info => ("INFO", "\x1b[32m"),   // green
                        log::Level::Debug => ("DEBUG", "\x1b[36m"), // cyan
                        log::Level::Trace => ("TRACE", "\x1b[35m"), // magenta
                    };

                    writeln!(buf, "{}[{level}]\x1b[0m {}", color, record.args())
                });

            let _ = builder.try_init();
        });
    }

    fn format_download(n: u64) -> String {
        if n >= 1_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else if n >= 1_000 {
            format!("{:.1}k", n as f64 / 1_000.0)
        } else {
            format!("{}", n)
        }
    }

    fn build_progress_bar(total: u64) -> ProgressBar {
        let pb = ProgressBar::new(total);
        let style = ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] |{bar:70.white/white}| {msg} {eta}",
        )
        .expect("valid progress template");

        pb.set_style(style);
        pb.set_message(format!(
            "[{}/{}] fetching packages...",
            format_download(0),
            format_download(total)
        ));
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    }
}

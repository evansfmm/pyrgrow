use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use numpy::PyArray2;
use numpy::ToPyArray;
use pyo3::types::PyDict;
use pyo3::{prelude::*, types::PyType};

use rgrow::ffs;
use rgrow::ffs::FFSRunConfig;
use rgrow::simulation;
use rgrow::system::EvolveBounds;
use rgrow::tileset;
use rgrow::tileset::TileShape;

#[derive(FromPyObject)]
enum Ident {
    Num(usize),
    Name(String),
}

impl IntoPy<PyObject> for Ident {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Ident::Num(num) => num.into_py(py),
            Ident::Name(name) => name.into_py(py),
        }
    }
}

impl From<Ident> for tileset::GlueIdent {
    fn from(ident: Ident) -> Self {
        match ident {
            Ident::Num(num) => tileset::GlueIdent::Num(num),
            Ident::Name(name) => tileset::GlueIdent::Name(name),
        }
    }
}

impl From<Ident> for tileset::TileIdent {
    fn from(ident: Ident) -> Self {
        match ident {
            Ident::Num(num) => tileset::TileIdent::Num(num),
            Ident::Name(name) => tileset::TileIdent::Name(name),
        }
    }
}

impl From<tileset::GlueIdent> for Ident {
    fn from(ident: tileset::GlueIdent) -> Self {
        match ident {
            tileset::GlueIdent::Num(num) => Ident::Num(num),
            tileset::GlueIdent::Name(name) => Ident::Name(name),
        }
    }
}

impl From<tileset::TileIdent> for Ident {
    fn from(ident: tileset::TileIdent) -> Self {
        match ident {
            tileset::TileIdent::Num(num) => Ident::Num(num),
            tileset::TileIdent::Name(name) => Ident::Name(name),
        }
    }
}

impl From<tileset::Tile> for Tile {
    fn from(tile: tileset::Tile) -> Self {
        Tile(tile)
    }
}

#[pyclass]
pub struct Tile(tileset::Tile);

#[pymethods]
impl Tile {
    #[new]
    fn new(
        edges: Vec<Ident>,
        name: Option<String>,
        stoic: Option<f64>,
        color: Option<String>,
        _shape: Option<String>,
    ) -> Self {
        let edges: Vec<tileset::GlueIdent> = edges.into_iter().map(|e| e.into()).collect();
        let tile = tileset::Tile {
            name,
            edges,
            stoic,
            color,
            shape: Some(TileShape::Single),
        };
        Tile(tile)
    }

    #[getter]
    fn get_name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    #[setter]
    fn set_name(&mut self, name: Option<String>) {
        self.0.name = name;
    }

    #[getter]
    fn get_stoic(&self) -> Option<f64> {
        self.0.stoic
    }

    #[setter]
    fn set_stoic(&mut self, stoic: Option<f64>) {
        self.0.stoic = stoic;
    }

    #[getter]
    fn get_color(&self) -> Option<&str> {
        self.0.color.as_deref()
    }

    #[setter]
    fn set_color(&mut self, color: Option<String>) {
        self.0.color = color;
    }

    #[getter]
    fn get_edges(&self) -> Vec<Ident> {
        self.0.edges.iter().map(|e| e.clone().into()).collect()
    }

    #[setter]
    fn set_edges(&mut self, edges: Vec<Ident>) {
        self.0.edges = edges.into_iter().map(|e| e.into()).collect();
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

#[pyclass]
pub struct TileSet(Arc<RwLock<tileset::TileSet>>);

impl TileSet {
    fn read(&self) -> PyResult<std::sync::RwLockReadGuard<'_, tileset::TileSet>> {
        let x = self
            .0
            .read()
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))?;
        Ok(x)
    }

    fn write(&self) -> PyResult<std::sync::RwLockWriteGuard<'_, tileset::TileSet>> {
        let x = self
            .0
            .write()
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))?;
        Ok(x)
    }
}

#[pymethods]
impl TileSet {
    #[classmethod]
    fn from_json(_cls: &PyType, data: &str) -> PyResult<Self> {
        let tileset = tileset::TileSet::from_json(data);
        match tileset {
            Ok(tileset) => Ok(TileSet(Arc::new(RwLock::new(tileset)))),
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                err.to_string(),
            )),
        }
    }

    /// Creates a TileSet from a dict by exporting to json, then parsing the json.
    #[classmethod]
    fn from_dict(_cls: &PyType, data: PyObject) -> PyResult<Self> {
        let json: String = Python::with_gil(|py| {
            let json = PyModule::import(py, "json")?;
            json.call_method1("dumps", (data,))?.extract::<String>()
        })?;

        let tileset = tileset::TileSet::from_json(&json);
        match tileset {
            Ok(tileset) => Ok(TileSet(Arc::new(RwLock::new(tileset)))),
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                err.to_string(),
            )),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn to_simulation(&self) -> PyResult<Simulation> {
        let sim = self.write()?.into_simulation();
        match sim {
            Ok(sim) => Ok(Simulation(RwLock::new(sim))),
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                err.to_string(),
            )),
        }
    }

    #[cfg(feature = "ui")]
    fn run_window(&self) -> PyResult<Simulation> {
        let f = self.read()?;
        let s = rgrow::ui::run_window(&f);

        let s2 =
            s.map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))?;
        Ok(Simulation(RwLock::new(s2)))
    }

    #[getter]
    fn get_tiles(&self) -> PyResult<Vec<Tile>> {
        Ok(self
            .read()?
            .tiles
            .iter()
            .map(|t| t.clone().into())
            .collect())
    }

    #[allow(clippy::too_many_arguments)]
    #[args(config = "FFSRunConfig::default()", kwargs = "**")]
    fn run_ffs(
        &self,
        config: FFSRunConfig,
        kwargs: Option<&PyDict>,
        py: Python<'_>,
    ) -> PyResult<FFSResult> {
        let mut c = config;

        if let Some(dict) = kwargs {
            for (k, v) in dict.iter() {
                c._py_set(&k.extract::<String>()?, v, py)?;
            }
        }

        let res = py.allow_threads(|| self.0.read().unwrap().run_ffs(&c));
        match res {
            Ok(res) => Ok(FFSResult(res.into())),
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                err.to_string(),
            )),
        }
    }
}

#[pyclass]
pub struct Simulation(RwLock<Box<dyn simulation::Simulation>>);

#[pyclass]
pub struct TileColorView(Arc<RwLock<dyn simulation::Simulation>>);

#[pymethods]
impl Simulation {
    /// Evolves an individual state within bounds.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "($self, state_index, for_events, for_time, size_min, size_max, for_wall_time)"
    )]
    fn evolve(
        &mut self,
        state_index: Option<usize>,
        for_events: Option<u64>,
        total_events: Option<u64>,
        for_time: Option<f64>,
        total_time: Option<f64>,
        size_min: Option<u32>,
        size_max: Option<u32>,
        for_wall_time: Option<f64>,
        py: Python<'_>,
    ) -> PyResult<()> {
        if self.0.read().unwrap().n_states() == 0 {
            self.0
                .write()
                .unwrap()
                .add_state()
                .map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))?;
        }

        let bounds = EvolveBounds {
            for_events,
            for_time,
            total_events,
            total_time,
            size_min,
            size_max,
            for_wall_time: for_wall_time.map(Duration::from_secs_f64),
        };

        if !bounds.is_weakly_bounded() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "No bounds specified.",
            ));
        }

        py.allow_threads(|| {
            self.0
                .write()
                .unwrap()
                .evolve(state_index.unwrap_or(0), bounds)
                .map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))
                .map(|_x| ())
        })
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "use_rayon")]
    fn evolve_all(
        &mut self,
        for_events: Option<u64>,
        total_events: Option<u64>,
        for_time: Option<f64>,
        total_time: Option<f64>,
        size_min: Option<u32>,
        size_max: Option<u32>,
        for_wall_time: Option<f64>,
        py: Python<'_>,
    ) -> PyResult<()> {
        let bounds = EvolveBounds {
            for_events,
            for_time,
            total_events,
            total_time,
            size_min,
            size_max,
            for_wall_time: for_wall_time.map(Duration::from_secs_f64),
        };

        if !bounds.is_weakly_bounded() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "No bounds specified.",
            ));
        }

        py.allow_threads(|| self.0.write().unwrap().evolve_all(bounds)); // FIXME: handle errors

        Ok(())
    }

    /// Returns the current canvas for state_index (default 0), as an array copy.
    #[pyo3(text_signature = "($self, state_index)")]
    fn canvas<'py>(&self, state_index: Option<usize>, py: Python<'py>) -> &'py PyArray2<usize> {
        self.0
            .read()
            .unwrap()
            .state_ref(state_index.unwrap_or(0))
            .raw_array()
            .to_pyarray(py)
    }

    fn state_ntiles(&self, state_index: Option<usize>) -> u32 {
        self.0
            .read()
            .unwrap()
            .state_ref(state_index.unwrap_or(0))
            .ntiles()
    }

    fn state_time(&self, state_index: Option<usize>) -> f64 {
        self.0
            .read()
            .unwrap()
            .state_ref(state_index.unwrap_or(0))
            .time()
    }

    fn state_total_events(&self, state_index: Option<usize>) -> u64 {
        self.0
            .read()
            .unwrap()
            .state_ref(state_index.unwrap_or(0))
            .total_events()
    }

    /// Add a new state to the simulation.
    #[pyo3(text_signature = "($self, shape)")]
    fn add_state(&mut self) -> PyResult<usize> {
        self.0
            .write()
            .unwrap()
            .add_state()
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))
    }

    #[pyo3(text_signature = "($self, n, shape")]
    fn add_n_states(&mut self, n: usize) -> PyResult<Vec<usize>> {
        self.0
            .write()
            .unwrap()
            .add_n_states(n)
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))
    }

    // fn erase_states(&mut self, state_index: Option<usize>) -> PyResult<()> {
    //     self.0
    //         .write()
    //         .unwrap()
    //         .erase_states(state_index.unwrap_or(0))
    //         .map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))
    //         .map(|_x| ())
    // }

    #[getter]
    fn get_tile_concs(&self) -> PyResult<Vec<f64>> {
        Ok(self.0.read().unwrap().tile_concs())
    }

    #[getter]
    fn get_tile_stoics(&self) -> PyResult<Vec<f64>> {
        Ok(self.0.read().unwrap().tile_stoics())
    }
}

#[pyclass]
pub struct FFSResult(pub(crate) Arc<Box<dyn ffs::FFSResult>>);

#[pymethods]
impl FFSResult {
    /// Nucleation rate, in M/s.  Calculated from the forward probability vector,
    /// and dimerization rate.
    #[getter]
    fn get_nucleation_rate(&self) -> f64 {
        self.0.nucleation_rate()
    }

    #[getter]
    fn get_forward_vec(&self) -> Vec<f64> {
        self.0.forward_vec().clone()
    }

    #[getter]
    fn get_dimerization_rate(&self) -> f64 {
        self.0.dimerization_rate()
    }

    #[getter]
    fn get_surfaces(&self) -> Vec<FFSLevel> {
        self.0
            .surfaces()
            .iter()
            .enumerate()
            .map(|(i, _)| FFSLevel {
                res: self.0.clone(),
                level: i,
            })
            .collect()
    }

    fn __str__(&self) -> String {
        format!(
            "FFSResult({:1.4e} M/s, {:?})",
            self.0.nucleation_rate(),
            self.0.forward_vec()
        )
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }

    #[getter]
    fn previous_indices(&self) -> Vec<Vec<usize>> {
        self.get_surfaces()
            .iter()
            .map(|x| x.get_previous_indices())
            .collect()
    }
}

#[pyclass]
pub struct FFSLevel {
    res: Arc<Box<dyn ffs::FFSResult>>,
    level: usize,
}

#[pymethods]
impl FFSLevel {
    #[getter]
    fn get_configs<'py>(&self, py: Python<'py>) -> Vec<&'py PyArray2<usize>> {
        self.res.surfaces()[self.level]
            .configs()
            .iter()
            .map(|x| x.to_pyarray(py))
            .collect()
    }

    #[getter]
    fn get_previous_indices(&self) -> Vec<usize> {
        self.res.surfaces()[self.level].previous_list()
    }
}

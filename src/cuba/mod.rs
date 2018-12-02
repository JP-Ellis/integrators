//! Wrappers for Cuba integration routines. See Cuba documentation
//! [here](http://www.feynarts.de/cuba/) for details and installation
//! instructions.
//!
//! This module, and all its re-exports from the top-level `integrators`
//! module, are gated with the `cuba` feature. So, if you don't want to use
//! these wrappers, or don't have Cuba installed, just turn off that feature.

use std::{error, fmt, slice, vec};
use std::convert::From;
use std::os::raw::{c_int, c_longlong, c_void};

use super::traits::{IntegrandInput, IntegrandOutput};
use super::{IntegrationResult, Real};
use super::ffi::LandingPad;

mod cuhre;
pub use self::cuhre::Cuhre;

mod suave;
pub use self::suave::Suave;

mod vegas;
pub use self::vegas::Vegas;

unsafe extern "C"
fn cuba_integrand<A, B, F>(ndim: *const c_int,
                           x: *const Real,
                           ncomp: *const c_int,
                           f: *mut Real,
                           userdata: *mut c_void) -> c_int
    where A: IntegrandInput,
          B: IntegrandOutput,
          F: FnMut(A) -> B
{
    let fnptr = userdata as *mut LandingPad<A, B, F>;
    let lp: &mut LandingPad<A, B, F> = &mut *fnptr;

    let args = slice::from_raw_parts(x, *ndim as usize);
    let output = slice::from_raw_parts_mut(f, *ncomp as usize);

    match lp.try_call(args, output) {
        Ok(_) => 0,
        // -999 is special `abort` code to Cuba
        Err(_) => -999,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RandomNumberSource {
    Sobol,
    MersenneTwister,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CubaIntegrationResult {
    pub value: Real,
    pub error: Real,
    pub prob: Real
}

#[derive(Clone, Debug, PartialEq)]
pub struct CubaIntegrationResults {
    pub nregions: Option<c_int>,
    pub neval: c_longlong,
    pub results: Vec<CubaIntegrationResult>
}

#[derive(Clone, Debug, PartialEq)]
pub enum CubaError {
    /// The integrand input's dimensions are not supported by the given
    /// algorithm. The name of the algorithm and the number of dimensions
    /// attempted are given.
    BadDim(&'static str, usize),
    /// The integrand output's dimensions are not supported by the given
    /// algorithm. The name of the algorithm and the number of dimensions
    /// attempted are given.
    BadComp(&'static str, usize),
    /// The integration did not converge. Though the results did not reach
    /// the desired uncertainty, they still might be useful, and so are
    /// provided.
    DidNotConverge(CubaIntegrationResults),
}

impl fmt::Display for CubaError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::CubaError::*;
        match &self {
            &BadDim(name, ndim) => {
                write!(fmt, "invalid number of dimensions for algorithm {}: {}",
                       name, ndim)
            },
            &BadComp(name, ncomp) => {
                write!(fmt, "invalid number of outputs for algorithm {}: {}",
                       name, ncomp)
            },
            &DidNotConverge(_) => write!(fmt, "integral did not converge")
        }
    }
}

impl error::Error for CubaError {}

pub struct CubaResultsIter {
    iter: vec::IntoIter<CubaIntegrationResult>
}

impl Iterator for CubaResultsIter {
    type Item = IntegrationResult;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|CubaIntegrationResult { value, error, .. }| {
            IntegrationResult {
                value, error
            }
        })
    }
}

impl From<Vec<CubaIntegrationResult>> for CubaResultsIter {
    fn from(v: Vec<CubaIntegrationResult>) -> Self {
        CubaResultsIter {
            iter: v.into_iter()
        }
    }
}

impl super::traits::IntegrationResults for CubaIntegrationResults {
    type Iterator = CubaResultsIter;
    fn results(self) -> CubaResultsIter {
        From::from(self.results)
    }
}

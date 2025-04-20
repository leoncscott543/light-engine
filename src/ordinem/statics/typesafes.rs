// typesafes.rs
// This file defines a collection of strongly-typed wrappers around `f64` to represent various physical quantities, constants, and units. 
// These types provide clarity and type safety when working with scientific and engineering calculations.

/// Fundamental SI Units
pub struct Meters(pub f64); // Represents length in meters.
pub struct Kilograms(pub f64); // Represents mass in kilograms.
pub struct Seconds(pub f64); // Represents time in seconds.
pub struct Amperes(pub f64); // Represents electric current in amperes.
pub struct Kelvin(pub f64); // Represents temperature in kelvin.
pub struct Moles(pub f64); // Represents the amount of substance in moles.
pub struct Candela(pub f64); // Represents luminous intensity in candela.

/// Derived SI Units
pub struct Hertz(pub f64); // Represents frequency in hertz (1/s).
pub struct Newtons(pub f64); // Represents force in newtons (kg·m/s²).
pub struct NewtonMeters(pub f64); // Represents torque in newton-meters (N·m).
pub struct Torque(pub f64); // Represents torque in newton-meters (N·m).
pub struct JoulesPerSecond(pub f64); // Represents power in joules per second (W).
pub struct Joules(pub f64); // Represents energy in joules (N·m).
pub struct Watts(pub f64); // Represents power in watts (J/s).
pub struct Pascals(pub f64); // Represents pressure in pascals (N/m²).
pub struct Coulombs(pub f64); // Represents electric charge in coulombs (A·s).
pub struct ElectronVolts(pub f64);
pub struct Volts(pub f64); // Represents electric potential in volts (W/A).
pub struct Ohms(pub f64); // Represents electric resistance in ohms (V/A).
pub struct Farads(pub f64); // Represents capacitance in farads (C/V).
pub struct Henries(pub f64); // Represents inductance in henries (V·s/A).
pub struct Teslas(pub f64); // Represents magnetic field strength in teslas (N/(A·m)).
pub struct Webers(pub f64); // Represents magnetic flux in webers (T·m²).
pub struct Radians(pub f64); // Represents angle in radians.
pub struct Steradians(pub f64); // Represents solid angle in steradians.

/// Kinematics
pub struct MetersPerSecond(pub f64); // Represents velocity in meters per second.
pub struct MetersPerSecondSquared(pub f64); // Represents acceleration in meters per second squared.
pub struct RadiansPerSecond(pub f64); // Represents angular velocity in radians per second.
pub struct RadiansPerSecondSquared(pub f64); // Represents angular acceleration in radians per second squared.
pub struct Momentum(pub f64); // Represents momentum in kg·m/s.
pub struct Impulse(pub f64); // Represents impulse in N·s.
pub struct AngularMomentum(pub f64); // Represents angular momentum in kg·m²/s.
pub struct MomentOfInertia(pub f64); // Represents moment of inertia in kg·m².

/// Thermodynamics
pub struct Entropy(pub f64); // Represents entropy in joules per kelvin (J/K).
pub struct SpecificHeatCapacity(pub f64); // Represents specific heat capacity in J/(kg·K).
pub struct ThermalConductivity(pub f64); // Represents thermal conductivity in W/(m·K).
pub struct BoltzmannConstant(pub f64); // Represents the Boltzmann constant in J/K.
pub struct GasConstant(pub f64); // Represents the gas constant in J/(mol·K).
pub struct Temperature(pub f64); // Represents temperature in kelvin.
pub struct ThermalResistance(pub f64); // Represents thermal resistance in K/W.
pub struct ThermalExpansionCoefficient(pub f64); // Represents thermal expansion coefficient in 1/K.
pub struct Enthalpy(pub f64); // Represents enthalpy in joules.
pub struct GibbsFreeEnergy(pub f64); // Represents Gibbs free energy in joules.
pub struct HelmholtzFreeEnergy(pub f64); // Represents Helmholtz free energy in joules.

/// Electromagnetism
pub struct ElectricField(pub f64); // Represents electric field strength in volts per meter (V/m).
pub struct MagneticField(pub f64); // Represents magnetic field strength in teslas (T).
pub struct MagneticFlux(pub f64); // Represents magnetic flux in webers (Wb).
pub struct WeberPerSquareMeter(pub f64); // Represents magnetic flux density in teslas (T).
pub struct Capacitance(pub f64); // Represents capacitance in farads (F).
pub struct Inductance(pub f64); // Represents inductance in henries (H).
pub struct ElectronCharge(pub f64); // Represents the charge of an electron in coulombs.
pub struct Gauss(pub f64); // Represents magnetic field strength in gauss (1 G = 10^-4 T).

/// Optics
pub struct Lumens(pub f64); // Represents luminous flux in lumens (lm).
pub struct Lux(pub f64); // Represents illuminance in lux (lm/m²).
pub struct RefractiveIndex(pub f64); // Represents the refractive index (dimensionless).
pub struct FocalLength(pub f64); // Represents focal length in meters.
pub struct Wavelength(pub f64); // Represents wavelength in meters.
pub struct PhotonEnergy(pub f64); // Represents photon energy in joules.
pub struct OpticalPower(pub f64); // Represents optical power in diopters (1/m).

/// Relativity
pub struct LorentzFactor(pub f64); // Represents the Lorentz factor (dimensionless).
pub struct SpacetimeInterval(pub f64); // Represents spacetime interval in meters.
pub struct Energy(pub f64); // Represents energy in joules.
pub struct MassEnergy(pub f64); // Represents mass-energy equivalence in joules.
pub struct SchwarzschildRadius(pub f64); // Represents Schwarzschild radius in meters.

/// Quantum Mechanics
pub struct PlanckConstant(pub f64); // Represents Planck's constant in J·s.
pub struct ReducedPlanckConstant(pub f64); // Represents reduced Planck's constant (ħ) in J·s.
pub struct WaveNumber(pub f64); // Represents wave number in 1/m.
pub struct QuantumState(pub f64); // Represents a quantum state (dimensionless).
pub struct DeBroglieWavelength(pub f64); // Represents de Broglie wavelength in meters.
pub struct Probability(pub f64); // Represents probability (dimensionless).
pub struct WaveFunction(pub f64); // Represents a wave function (dimensionless).
pub struct Spin(pub f64); // Represents spin (dimensionless).
pub struct QuantumNumber(pub f64); // Represents a quantum number (dimensionless).
pub struct EnergyLevel(pub f64); // Represents energy level in joules.
pub struct ProbabilityDensity(pub f64); // Represents probability density in 1/m³.

/// Astronomy
pub struct LightYears(pub f64); // Represents distance in light-years.
pub struct Parsecs(pub f64); // Represents distance in parsecs.
pub struct AstronomicalUnits(pub f64); // Represents distance in astronomical units (AU).
pub struct SolarMasses(pub f64); // Represents mass in solar masses.
pub struct HubbleConstant(pub f64); // Represents the Hubble constant in (km/s)/Mpc.
pub struct Redshift(pub f64); // Represents redshift (dimensionless).
pub struct OrbitalPeriod(pub f64); // Represents orbital period in seconds.
pub struct Luminosity(pub f64); // Represents luminosity in watts.
pub struct ApparentMagnitude(pub f64); // Represents apparent magnitude (dimensionless).
pub struct AbsoluteMagnitude(pub f64); // Represents absolute magnitude (dimensionless).
pub struct Parallax(pub f64); // Represents parallax angle in arcseconds.
pub struct CosmicMicrowaveBackgroundTemperature(pub f64); // Represents CMB temperature in kelvin.
pub struct CriticalDensity(pub f64); // Represents critical density in kg/m³.
pub struct DarkMatterDensity(pub f64); // Represents dark matter density in kg/m³.
pub struct DarkEnergyDensity(pub f64); // Represents dark energy density in kg/m³.
pub struct ScaleFactor(pub f64); // Represents the scale factor of the universe (dimensionless).

/// Planck Units
pub struct PlanckLength(pub f64); // Represents Planck length in meters.
pub struct PlanckTime(pub f64); // Represents Planck time in seconds.
pub struct PlanckMass(pub f64); // Represents Planck mass in kilograms.
pub struct PlanckTemperature(pub f64); // Represents Planck temperature in kelvin.
pub struct PlanckCharge(pub f64); // Represents Planck charge in coulombs.
pub struct PlanckEnergy(pub f64); // Represents Planck energy in joules.
pub struct PlanckVolume(pub f64); // Represents Planck volume in cubic meters.
pub struct PlanckMomentum(pub f64); // Represents Planck momentum in kg·m/s.
pub struct PlanckPower(pub f64); // Represents Planck power in watts.
pub struct PlanckForce(pub f64); // Represents Planck force in newtons.
pub struct PlanckDensity(pub f64); // Represents Planck density in kg/m³.

/// Fluid Dynamics
pub struct ReynoldsNumber(pub f64); // Represents Reynolds number (dimensionless).
pub struct MachNumber(pub f64); // Represents Mach number (dimensionless).

/// Material Properties
pub struct Density(pub f64); // Represents density in kg/m³.
pub struct Volume(pub f64); // Represents volume in cubic meters.
pub struct Area(pub f64); // Represents area in square meters.
pub struct Length(pub f64); // Represents length in meters.

/// Non-SI Units
pub struct Liters(pub f64); // Represents volume in liters.
pub struct Bar(pub f64); // Represents pressure in bar (1 bar = 100,000 Pa).
pub struct Atmospheres(pub f64); // Represents pressure in atmospheres (1 atm = 101,325 Pa).
pub struct Calories(pub f64); // Represents energy in calories (1 cal = 4.184 J).
pub struct Dynes(pub f64); // Represents force in dynes (1 dyne = 10^-5 N).
pub struct Ergs(pub f64); // Represents energy in ergs (1 erg = 10^-7 J).

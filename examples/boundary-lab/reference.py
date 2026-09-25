"""Independent NumPy grid oracle and SciPy continuous localization check."""
import json
import pathlib
import subprocess

import numpy as np
from scipy.optimize import least_squares

ROOT = pathlib.Path(__file__).resolve().parents[2]


def cli(*args):
    result = subprocess.run(
        ["node", str(ROOT / "cli/bin/cli.js"), "boundary-lab", *args],
        capture_output=True, text=True, check=True, timeout=30,
    )
    return json.loads(result.stdout)


plan = cli("plan")
report = cli("run", "--protocol-hash", plan["protocolHash"])["report"]
config = plan["protocol"]["inference"]
sensors = report["dataset"]["sensors"]
positions = np.array([sensor["positionM"] for sensor in sensors])
offsets = np.array([sensor["offsetDb"] for sensor in sensors])
axis = np.linspace(config["gridMinimumM"], config["gridMaximumM"], 33)
grid = np.array([(x, y) for x in axis for y in axis])
attenuation = 20 * np.log10(np.linalg.norm(grid[:, None, :] - positions[None, :, :], axis=2))
checked = 0
maximum_residual_delta = 0.0
maximum_grid_to_continuous_m = 0.0
for trial, prediction in zip(report["dataset"]["trials"], report["predictionBundle"]["predictions"]):
    for channel, predicted in zip(trial["channels"], prediction["channels"]):
        if predicted["sensorsAboveThreshold"] < config["minimumSensors"]:
            continue
        received = np.array([np.nan if value is None else value for value in channel["rssiDbm"]]) - offsets
        selected = np.isfinite(received) & (received > config["thresholdDbm"])
        powers = received[selected][None, :] + attenuation[:, selected]
        means = powers.mean(axis=1)
        residuals = np.sqrt(((powers - means[:, None]) ** 2).mean(axis=1))
        best = int(np.argmin(residuals))
        delta = abs(float(residuals[best]) - predicted["bestFitResidualDb"])
        maximum_residual_delta = max(maximum_residual_delta, delta)
        assert delta < 1e-10
        if predicted["status"] == "detected":
            assert np.linalg.norm(grid[best] - predicted["estimate"]["positionM"]) < 1e-10
            assert abs(means[best] - predicted["estimate"]["powerAtOneMetreDbm"]) < 1e-10
            # Start independently at room centre, not at the JavaScript grid solution.
            def residual(parameters):
                distances = np.linalg.norm(positions[selected] - parameters[:2], axis=1)
                return parameters[2] - 20 * np.log10(distances) - received[selected]

            fit = least_squares(residual, [1, 1, -44], bounds=([0.2, 0.2, -55], [1.8, 1.8, -30]),
                                ftol=1e-12, xtol=1e-12, gtol=1e-12)
            assert fit.success
            assert np.sqrt(np.mean(fit.fun ** 2)) <= residuals[best] + 1e-9
            spatial_delta = float(np.linalg.norm(fit.x[:2] - grid[best]))
            maximum_grid_to_continuous_m = max(maximum_grid_to_continuous_m, spatial_delta)
            assert spatial_delta < 0.08
        else:
            assert residuals[best] > config["maximumResidualDb"] or not (
                config["minimumPowerAtOneMetreDbm"] <= means[best] <= config["maximumPowerAtOneMetreDbm"])
        checked += 1

assert checked == 299
print(json.dumps({"schema": "ruqu.boundary.reference.v1", "cases": checked,
                  "maximumResidualDeltaDb": maximum_residual_delta,
                  "maximumGridToContinuousM": maximum_grid_to_continuous_m,
                  "scope": "Independent numerical implementation, not independent physical validation"}, indent=2))

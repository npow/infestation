# infestation-gym

### Overview
- **Environment ID**: `infestation-gym`
- **Short description**: OpenEnv-backed Infestation puzzle environment using `vf.OpenEnvEnv`.
- **Tags**: openenv, gym, puzzle

### Structure
- `infestation_gym/` contains the Verifiers wrapper.
- `proj/` contains the OpenEnv FastAPI project.
- `proj/server/Dockerfile` builds the real Infestation solver into the sandbox image.
- `proj/.build.json` is produced by `vf-build` after the image is pushed to Prime.

### Quickstart
Build and register the Prime sandbox image:

```bash
uv run vf-build infestation-gym
```

Load the Verifiers environment after a successful build:

```python
import infestation_gym

env = infestation_gym.load_environment()
```

# Infestation OpenEnv Server

This folder contains the FastAPI/OpenEnv server used by the `infestation-gym`
Verifiers package.

Required files:
- `openenv.yaml`
- `pyproject.toml`
- `server/Dockerfile`
- `server/app.py`

Build the sandbox image from the parent environments directory:

```bash
prime --plain env build infestation-gym --path environments
```

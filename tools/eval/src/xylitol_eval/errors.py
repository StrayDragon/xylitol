"""Typed errors and process exit codes."""

EXIT_OK = 0
EXIT_USAGE = 1
EXIT_INFRA = 2
EXIT_GRADE = 3


class EvalError(Exception):
    """Base error for eval harness."""


class ConfigError(EvalError):
    """Invalid or missing eval configuration."""


class PhaseNotImplemented(EvalError):
    """The requested phase is not implemented yet."""


class InfraError(EvalError):
    """Docker, dataset, or other infrastructure failure."""

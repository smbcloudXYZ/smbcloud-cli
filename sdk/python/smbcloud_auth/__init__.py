from __future__ import annotations

import json
from dataclasses import dataclass
from enum import Enum
from importlib.metadata import PackageNotFoundError, version
from typing import Any, Dict

from ._native import NativeSdkError
from ._native import login_with_client as _login_with_client
from ._native import logout_with_client as _logout_with_client
from ._native import me_with_client as _me_with_client
from ._native import remove_with_client as _remove_with_client
from ._native import signup_with_client as _signup_with_client


class Environment(str, Enum):
    DEV = "dev"
    PRODUCTION = "production"


class SdkError(Exception):
    def __init__(self, error_code: int, error_name: str, message: str) -> None:
        super().__init__(message)
        self.error_code = error_code
        self.error_name = error_name
        self.message = message


@dataclass(frozen=True)
class AuthClient:
    env: Environment
    app_id: str
    app_secret: str

    def signup(self, email: str, password: str) -> Dict[str, Any]:
        return signup_with_client(self.env, self.app_id, self.app_secret, email, password)

    def login(self, email: str, password: str) -> Dict[str, Any]:
        return login_with_client(self.env, self.app_id, self.app_secret, email, password)

    def logout(self, access_token: str) -> None:
        logout_with_client(self.env, self.app_id, self.app_secret, access_token)

    def me(self, access_token: str) -> Dict[str, Any]:
        return me_with_client(self.env, self.app_id, self.app_secret, access_token)

    def remove(self, access_token: str) -> None:
        remove_with_client(self.env, self.app_id, self.app_secret, access_token)


def signup_with_client(
    env: Environment | str,
    app_id: str,
    app_secret: str,
    email: str,
    password: str,
) -> Dict[str, Any]:
    return _run(_signup_with_client, env, app_id, app_secret, email, password)


def login_with_client(
    env: Environment | str,
    app_id: str,
    app_secret: str,
    email: str,
    password: str,
) -> Dict[str, Any]:
    return _run(_login_with_client, env, app_id, app_secret, email, password)


def logout_with_client(
    env: Environment | str,
    app_id: str,
    app_secret: str,
    access_token: str,
) -> None:
    _run(_logout_with_client, env, app_id, app_secret, access_token)


def me_with_client(
    env: Environment | str,
    app_id: str,
    app_secret: str,
    access_token: str,
) -> Dict[str, Any]:
    return _run(_me_with_client, env, app_id, app_secret, access_token)


def remove_with_client(
    env: Environment | str,
    app_id: str,
    app_secret: str,
    access_token: str,
) -> None:
    _run(_remove_with_client, env, app_id, app_secret, access_token)


def _run(func: Any, env: Environment | str, *args: str) -> Any:
    try:
        return func(_normalize_env(env), *args)
    except NativeSdkError as exc:
        raise _sdk_error_from_native(exc) from exc


def _normalize_env(env: Environment | str) -> str:
    if isinstance(env, Environment):
        return env.value
    return Environment(env).value


def _sdk_error_from_native(exc: NativeSdkError) -> SdkError:
    try:
        payload = json.loads(str(exc))
    except json.JSONDecodeError:
        return SdkError(0, "Unknown", str(exc))

    return SdkError(
        int(payload.get("error_code", 0)),
        str(payload.get("error_name", "Unknown")),
        str(payload.get("message", str(exc))),
    )


try:
    __version__ = version("smbcloud-sdk-auth")
except PackageNotFoundError:
    __version__ = "0.0.0"


__all__ = [
    "AuthClient",
    "Environment",
    "SdkError",
    "__version__",
    "login_with_client",
    "logout_with_client",
    "me_with_client",
    "remove_with_client",
    "signup_with_client",
]

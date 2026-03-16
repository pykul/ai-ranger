from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Optional as _Optional

DESCRIPTOR: _descriptor.FileDescriptor

class EnrollmentRequest(_message.Message):
    __slots__ = ("token", "agent_id", "hostname", "os_username", "os", "agent_version")
    TOKEN_FIELD_NUMBER: _ClassVar[int]
    AGENT_ID_FIELD_NUMBER: _ClassVar[int]
    HOSTNAME_FIELD_NUMBER: _ClassVar[int]
    OS_USERNAME_FIELD_NUMBER: _ClassVar[int]
    OS_FIELD_NUMBER: _ClassVar[int]
    AGENT_VERSION_FIELD_NUMBER: _ClassVar[int]
    token: str
    agent_id: str
    hostname: str
    os_username: str
    os: str
    agent_version: str
    def __init__(self, token: _Optional[str] = ..., agent_id: _Optional[str] = ..., hostname: _Optional[str] = ..., os_username: _Optional[str] = ..., os: _Optional[str] = ..., agent_version: _Optional[str] = ...) -> None: ...

class EnrollmentResponse(_message.Message):
    __slots__ = ("org_id", "agent_id")
    ORG_ID_FIELD_NUMBER: _ClassVar[int]
    AGENT_ID_FIELD_NUMBER: _ClassVar[int]
    org_id: str
    agent_id: str
    def __init__(self, org_id: _Optional[str] = ..., agent_id: _Optional[str] = ...) -> None: ...

from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class DetectionMethod(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SNI: _ClassVar[DetectionMethod]
    DNS: _ClassVar[DetectionMethod]
    IP_RANGE: _ClassVar[DetectionMethod]
    TCP_HEURISTIC: _ClassVar[DetectionMethod]

class CaptureMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    DNS_SNI: _ClassVar[CaptureMode]
    MITM: _ClassVar[CaptureMode]
SNI: DetectionMethod
DNS: DetectionMethod
IP_RANGE: DetectionMethod
TCP_HEURISTIC: DetectionMethod
DNS_SNI: CaptureMode
MITM: CaptureMode

class AiConnectionEvent(_message.Message):
    __slots__ = ("agent_id", "machine_hostname", "os_username", "os_type", "timestamp_ms", "duration_ms", "provider", "provider_host", "model_hint", "process_name", "process_pid", "process_path", "connection_id", "detection_method", "capture_mode", "src_ip", "content_available", "payload_ref", "model_exact", "token_count_input", "token_count_output", "latency_ttfb_ms")
    AGENT_ID_FIELD_NUMBER: _ClassVar[int]
    MACHINE_HOSTNAME_FIELD_NUMBER: _ClassVar[int]
    OS_USERNAME_FIELD_NUMBER: _ClassVar[int]
    OS_TYPE_FIELD_NUMBER: _ClassVar[int]
    TIMESTAMP_MS_FIELD_NUMBER: _ClassVar[int]
    DURATION_MS_FIELD_NUMBER: _ClassVar[int]
    PROVIDER_FIELD_NUMBER: _ClassVar[int]
    PROVIDER_HOST_FIELD_NUMBER: _ClassVar[int]
    MODEL_HINT_FIELD_NUMBER: _ClassVar[int]
    PROCESS_NAME_FIELD_NUMBER: _ClassVar[int]
    PROCESS_PID_FIELD_NUMBER: _ClassVar[int]
    PROCESS_PATH_FIELD_NUMBER: _ClassVar[int]
    CONNECTION_ID_FIELD_NUMBER: _ClassVar[int]
    DETECTION_METHOD_FIELD_NUMBER: _ClassVar[int]
    CAPTURE_MODE_FIELD_NUMBER: _ClassVar[int]
    SRC_IP_FIELD_NUMBER: _ClassVar[int]
    CONTENT_AVAILABLE_FIELD_NUMBER: _ClassVar[int]
    PAYLOAD_REF_FIELD_NUMBER: _ClassVar[int]
    MODEL_EXACT_FIELD_NUMBER: _ClassVar[int]
    TOKEN_COUNT_INPUT_FIELD_NUMBER: _ClassVar[int]
    TOKEN_COUNT_OUTPUT_FIELD_NUMBER: _ClassVar[int]
    LATENCY_TTFB_MS_FIELD_NUMBER: _ClassVar[int]
    agent_id: str
    machine_hostname: str
    os_username: str
    os_type: str
    timestamp_ms: int
    duration_ms: int
    provider: str
    provider_host: str
    model_hint: str
    process_name: str
    process_pid: int
    process_path: str
    connection_id: str
    detection_method: DetectionMethod
    capture_mode: CaptureMode
    src_ip: str
    content_available: bool
    payload_ref: str
    model_exact: str
    token_count_input: int
    token_count_output: int
    latency_ttfb_ms: int
    def __init__(self, agent_id: _Optional[str] = ..., machine_hostname: _Optional[str] = ..., os_username: _Optional[str] = ..., os_type: _Optional[str] = ..., timestamp_ms: _Optional[int] = ..., duration_ms: _Optional[int] = ..., provider: _Optional[str] = ..., provider_host: _Optional[str] = ..., model_hint: _Optional[str] = ..., process_name: _Optional[str] = ..., process_pid: _Optional[int] = ..., process_path: _Optional[str] = ..., connection_id: _Optional[str] = ..., detection_method: _Optional[_Union[DetectionMethod, str]] = ..., capture_mode: _Optional[_Union[CaptureMode, str]] = ..., src_ip: _Optional[str] = ..., content_available: bool = ..., payload_ref: _Optional[str] = ..., model_exact: _Optional[str] = ..., token_count_input: _Optional[int] = ..., token_count_output: _Optional[int] = ..., latency_ttfb_ms: _Optional[int] = ...) -> None: ...

class EventBatch(_message.Message):
    __slots__ = ("agent_id", "sent_at_ms", "events")
    AGENT_ID_FIELD_NUMBER: _ClassVar[int]
    SENT_AT_MS_FIELD_NUMBER: _ClassVar[int]
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    agent_id: str
    sent_at_ms: int
    events: _containers.RepeatedCompositeFieldContainer[AiConnectionEvent]
    def __init__(self, agent_id: _Optional[str] = ..., sent_at_ms: _Optional[int] = ..., events: _Optional[_Iterable[_Union[AiConnectionEvent, _Mapping]]] = ...) -> None: ...

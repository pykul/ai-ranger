"""RabbitMQ publisher for event batches.

Publishes raw protobuf bytes to the ranger.events exchange with routing key 'ingest'.
All queue and exchange names come from constants.py.
Connection URL comes from the Settings class in config.py.
"""

import pika

from config import get_settings
from constants import (
    RABBITMQ_EXCHANGE,
    RABBITMQ_HEARTBEAT_SECS,
    RABBITMQ_PUBLISH_RETRIES,
    RABBITMQ_ROUTING_KEY,
)

_settings = get_settings()

_connection: pika.BlockingConnection | None = None
_channel: pika.adapters.blocking_connection.BlockingChannel | None = None


def _get_channel() -> pika.adapters.blocking_connection.BlockingChannel:
    """Get or create a RabbitMQ channel. Reconnects on failure."""
    global _connection, _channel
    if _connection is None or _connection.is_closed:
        params = pika.URLParameters(_settings.rabbitmq_url)
        params.heartbeat = RABBITMQ_HEARTBEAT_SECS
        _connection = pika.BlockingConnection(params)
        _channel = _connection.channel()
    assert _channel is not None
    return _channel


def _reset_connection() -> None:
    """Close and discard the current connection so the next call reconnects."""
    global _connection, _channel
    _channel = None
    if _connection is not None:
        try:
            _connection.close()
        except Exception:
            pass
        _connection = None


def publish_event_batch(payload: bytes) -> None:
    """Publish raw protobuf bytes to the events exchange.

    Retries once on connection failure — RabbitMQ may have closed the idle
    connection between heartbeats.

    Args:
        payload: Serialized EventBatch protobuf message.
    """
    for attempt in range(RABBITMQ_PUBLISH_RETRIES):
        try:
            channel = _get_channel()
            channel.basic_publish(
                exchange=RABBITMQ_EXCHANGE,
                routing_key=RABBITMQ_ROUTING_KEY,
                body=payload,
                properties=pika.BasicProperties(delivery_mode=2),  # persistent
            )
            return
        except (pika.exceptions.AMQPConnectionError, pika.exceptions.AMQPChannelError):
            _reset_connection()
            if attempt == RABBITMQ_PUBLISH_RETRIES - 1:
                raise

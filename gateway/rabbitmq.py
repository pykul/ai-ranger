"""RabbitMQ publisher for event batches.

Publishes raw protobuf bytes to the ranger.events exchange with routing key 'ingest'.
All queue and exchange names come from constants.py.
"""

import os

import pika

from constants import RABBITMQ_EXCHANGE, RABBITMQ_ROUTING_KEY

# Default matches docker-compose.yml rabbitmq service.
RABBITMQ_URL = os.environ.get("RABBITMQ_URL", "amqp://guest:guest@localhost:5672/")

_connection: pika.BlockingConnection | None = None
_channel: pika.adapters.blocking_connection.BlockingChannel | None = None


def _get_channel() -> pika.adapters.blocking_connection.BlockingChannel:
    """Get or create a RabbitMQ channel. Reconnects on failure."""
    global _connection, _channel
    if _connection is None or _connection.is_closed:
        params = pika.URLParameters(RABBITMQ_URL)
        _connection = pika.BlockingConnection(params)
        _channel = _connection.channel()
    assert _channel is not None
    return _channel


def publish_event_batch(payload: bytes) -> None:
    """Publish raw protobuf bytes to the events exchange.

    Args:
        payload: Serialized EventBatch protobuf message.
    """
    channel = _get_channel()
    channel.basic_publish(
        exchange=RABBITMQ_EXCHANGE,
        routing_key=RABBITMQ_ROUTING_KEY,
        body=payload,
        properties=pika.BasicProperties(delivery_mode=2),  # persistent
    )

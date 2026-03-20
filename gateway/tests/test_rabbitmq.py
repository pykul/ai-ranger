"""Unit tests for rabbitmq.py publish retry logic.

These tests mock pika to verify that publish_event_batch:
- publishes successfully on first attempt in the happy path
- reconnects and retries once when the connection is stale
- raises after exhausting retries
"""

from unittest.mock import MagicMock, patch

import pika.exceptions
import pytest

import rabbitmq


@pytest.fixture(autouse=True)
def _reset_rabbitmq_globals():
    """Reset module-level connection state before each test."""
    rabbitmq._connection = None
    rabbitmq._channel = None
    yield
    rabbitmq._connection = None
    rabbitmq._channel = None


@patch("rabbitmq.pika.BlockingConnection")
@patch("rabbitmq.pika.URLParameters")
def test_publish_success(mock_url_params, mock_conn_cls):
    """Happy path: publish succeeds on the first attempt."""
    mock_channel = MagicMock()
    mock_conn = MagicMock()
    mock_conn.is_closed = False
    mock_conn.channel.return_value = mock_channel
    mock_conn_cls.return_value = mock_conn

    rabbitmq.publish_event_batch(b"\x00\x01\x02")

    mock_channel.basic_publish.assert_called_once()
    call_kwargs = mock_channel.basic_publish.call_args
    assert call_kwargs[1]["body"] == b"\x00\x01\x02"


@patch("rabbitmq.pika.BlockingConnection")
@patch("rabbitmq.pika.URLParameters")
def test_publish_retries_on_stale_connection(mock_url_params, mock_conn_cls):
    """First publish fails with AMQPConnectionError, retry succeeds."""
    # First connection: basic_publish raises
    stale_channel = MagicMock()
    stale_channel.basic_publish.side_effect = pika.exceptions.AMQPConnectionError(
        "connection reset"
    )
    stale_conn = MagicMock()
    stale_conn.is_closed = False
    stale_conn.channel.return_value = stale_channel

    # Second connection: works
    fresh_channel = MagicMock()
    fresh_conn = MagicMock()
    fresh_conn.is_closed = False
    fresh_conn.channel.return_value = fresh_channel

    mock_conn_cls.side_effect = [stale_conn, fresh_conn]

    rabbitmq.publish_event_batch(b"\x00\x01\x02")

    # Stale channel should have been called and failed
    stale_channel.basic_publish.assert_called_once()
    # Fresh channel should have succeeded
    fresh_channel.basic_publish.assert_called_once()


@patch("rabbitmq.pika.BlockingConnection")
@patch("rabbitmq.pika.URLParameters")
def test_publish_raises_after_all_retries_exhausted(mock_url_params, mock_conn_cls):
    """Both attempts fail — exception propagates."""
    bad_channel = MagicMock()
    bad_channel.basic_publish.side_effect = pika.exceptions.AMQPConnectionError(
        "connection reset"
    )
    bad_conn = MagicMock()
    bad_conn.is_closed = False
    bad_conn.channel.return_value = bad_channel
    mock_conn_cls.return_value = bad_conn

    with pytest.raises(pika.exceptions.AMQPConnectionError):
        rabbitmq.publish_event_batch(b"\x00\x01\x02")


@patch("rabbitmq.pika.BlockingConnection")
@patch("rabbitmq.pika.URLParameters")
def test_publish_retries_on_channel_error(mock_url_params, mock_conn_cls):
    """AMQPChannelError also triggers reconnect."""
    stale_channel = MagicMock()
    stale_channel.basic_publish.side_effect = pika.exceptions.AMQPChannelError(
        "channel closed"
    )
    stale_conn = MagicMock()
    stale_conn.is_closed = False
    stale_conn.channel.return_value = stale_channel

    fresh_channel = MagicMock()
    fresh_conn = MagicMock()
    fresh_conn.is_closed = False
    fresh_conn.channel.return_value = fresh_channel
    mock_conn_cls.side_effect = [stale_conn, fresh_conn]

    rabbitmq.publish_event_batch(b"\x00\x01\x02")

    fresh_channel.basic_publish.assert_called_once()


def test_reset_connection_clears_state():
    """_reset_connection sets globals to None and tolerates close errors."""
    mock_conn = MagicMock()
    mock_conn.close.side_effect = Exception("already closed")
    rabbitmq._connection = mock_conn
    rabbitmq._channel = MagicMock()

    rabbitmq._reset_connection()

    assert rabbitmq._connection is None
    assert rabbitmq._channel is None

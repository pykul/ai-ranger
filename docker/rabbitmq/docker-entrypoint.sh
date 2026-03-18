#!/bin/sh
# Generate definitions.json from the template using environment variables.
# This ensures RabbitMQ credentials are controlled entirely by
# RABBITMQ_DEFAULT_USER and RABBITMQ_DEFAULT_PASS in .env.
#
# Uses sed instead of envsubst since gettext is not installed in the
# rabbitmq:3-management-alpine image.
set -e

sed -e "s/\${RABBITMQ_DEFAULT_USER}/${RABBITMQ_DEFAULT_USER}/g" \
    -e "s/\${RABBITMQ_DEFAULT_PASS}/${RABBITMQ_DEFAULT_PASS}/g" \
    /etc/rabbitmq/definitions.json.template > /etc/rabbitmq/definitions.json

# Execute the original RabbitMQ entrypoint with all arguments.
exec docker-entrypoint.sh "$@"

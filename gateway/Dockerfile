FROM python:3.12-slim

WORKDIR /app

COPY gateway/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy gateway source.
COPY gateway/ .

# Copy proto generated Python code so imports resolve.
COPY proto/gen/python /app/proto/

# Startup command is in docker-compose.yml:
#   sh -c "alembic upgrade head && uvicorn main:app --host 0.0.0.0 --port 8080"
# This keeps the Dockerfile simple and the startup sequence visible in compose.

EXPOSE 8080
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8080"]

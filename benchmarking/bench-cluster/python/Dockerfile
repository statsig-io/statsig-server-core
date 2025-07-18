FROM python:3.11-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

WORKDIR /app

RUN pip install --no-cache-dir aiohttp
RUN pip install --no-cache-dir statsig-python-core --pre

COPY . .

CMD ["sh", "run.sh"]

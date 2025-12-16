FROM python:3.11-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

WORKDIR /app

ARG RELEASE_TAG
ENV RELEASE_TAG=${RELEASE_TAG}

RUN apt-get update && apt-get install -y jq curl

COPY install.sh .
RUN chmod +x install.sh && ./install.sh

RUN pip install --no-cache-dir statsig

COPY . .

CMD ["sh", "run.sh"]

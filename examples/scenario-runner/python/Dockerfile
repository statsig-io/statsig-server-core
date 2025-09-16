FROM python:3.11-slim-bookworm

RUN apt-get update && apt-get install -y curl

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

WORKDIR /app

ARG CACHE_BUST=1

RUN CORE_VERSION=$(curl -s https://pypi.org/rss/project/statsig-python-core/releases.xml | grep -o '<title>.*b.[0-9].*</title>' | head -1 | sed -E 's#<title>(.*)</title>#\1#') \
    && pip install --no-cache-dir statsig-python-core==$CORE_VERSION
RUN pip install --no-cache-dir requests statsig

RUN IN=$CACHE_BUST pip list

COPY . .

CMD ["python", "main.py"]

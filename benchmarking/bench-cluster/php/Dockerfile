FROM --platform=linux/amd64 php:8.3-cli

ENV COMPOSER_ALLOW_SUPERUSER=1 COMPOSER_MEMORY_LIMIT=-1

RUN apt-get update && apt-get install -y --no-install-recommends git unzip jq ca-certificates \
 && rm -rf /var/lib/apt/lists/*

RUN php -r "copy('https://getcomposer.org/installer','composer-setup.php');" \
 && php composer-setup.php --install-dir=/usr/local/bin --filename=composer \
 && rm composer-setup.php \
 && composer --version \
 && composer clear-cache

RUN echo "ffi.enable=1" > /usr/local/etc/php/conf.d/zz-ffi.ini

WORKDIR /app
COPY . .
ARG RELEASE_TAG
ENV RELEASE_TAG=${RELEASE_TAG}

RUN composer install --no-interaction --no-progress --no-scripts

COPY install.sh /usr/local/bin/install.sh
RUN chmod +x /usr/local/bin/install.sh && /usr/local/bin/install.sh
RUN php --version

CMD ["php", "main.php"]

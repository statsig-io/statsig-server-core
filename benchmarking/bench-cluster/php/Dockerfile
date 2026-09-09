FROM composer:2.8.12@sha256:5248900ab8b5f7f880c2d62180e40960cd87f60149ec9a1abfd62ac72a02577c AS composer

# Preserve the PHP 7.4 benchmark baseline. Bullseye is archived; this is
# legacy compatibility coverage, not a security-supported runtime.
FROM php:7.4.33-cli-bullseye@sha256:620a6b9f4d4feef2210026172570465e9d0c1de79766418d3affd09190a7fda5

# The final PHP image predates the archive: no package downgrades or relaxed
# signature/expiry checks are needed when installing from its signed index.
RUN printf 'deb http://archive.debian.org/debian bullseye main\n' > /etc/apt/sources.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends unzip libzip-dev libffi-dev \
    && rm -rf /var/lib/apt/lists/*
RUN docker-php-ext-install zip ffi
COPY --from=composer /usr/bin/composer /usr/local/bin/composer

WORKDIR /app

RUN echo "ffi.enable=1" >> /usr/local/etc/php/conf.d/docker-php-ext-ffi.ini

COPY . .
RUN composer install
RUN php --version
RUN php -r 'FFI::cdef("int abs(int);"); echo "PHP benchmark image preflight: FFI ready\n";'

CMD ["php", "main.php"]

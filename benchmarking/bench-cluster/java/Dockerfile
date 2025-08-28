FROM gradle:8-jdk11 AS builder
WORKDIR /app

ARG RELEASE_TAG
ENV RELEASE_TAG=${RELEASE_TAG}
COPY build.gradle ./

RUN set -eux; \
    CORE_PROP=""; \
    if [ -n "$RELEASE_TAG" ]; then CORE_PROP="-PcoreTag=$RELEASE_TAG"; fi; \
    gradle dependencies --no-daemon $CORE_PROP
COPY . .
RUN set -eux; \
    CORE_PROP=""; \
    if [ -n "$RELEASE_TAG" ]; then CORE_PROP="-PcoreTag=$RELEASE_TAG"; fi; \
    gradle shadowJar --no-daemon $CORE_PROP

FROM eclipse-temurin:11
RUN java --version
WORKDIR /app
COPY --from=builder /app/build/libs/*.jar app.jar
COPY --from=builder /app/build/versions.properties /app/build/versions.properties
ENV JAVA_OPTS="-XX:+UseG1GC -Xms512m -Xmx2g"

# Run the application
CMD ["java", "-jar", "app.jar"]

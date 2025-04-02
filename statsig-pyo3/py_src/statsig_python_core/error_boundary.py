class ErrorBoundary:
    @staticmethod
    def wrap(instance):
        try:
            for name in get_all_instance_method_names(instance):
                original = getattr(instance, name)

                if hasattr(original, '_error_boundary_wrapped'):
                    continue

                def create_wrapped(method_name, orig_method):
                    def wrapped(*args, **kwargs):
                        return ErrorBoundary._capture(method_name, lambda: orig_method(*args, **kwargs))

                    wrapped._error_boundary_wrapped = True
                    return wrapped

                setattr(instance, name, create_wrapped(name, original))

        except Exception as err:
            ErrorBoundary._on_error('eb:wrap', err)

    @staticmethod
    def _capture(tag, task):
        try:
            return task()
        except Exception as error:
            ErrorBoundary._on_error(tag, error)
            return None

    @staticmethod
    def _on_error(tag, error):
        print(f"Statsig SDK Error (Python Bindings): {tag}", error)


def get_all_instance_method_names(instance):
    names = set()

    cls = instance.__class__
    classes = [cls]

    parent = cls.__base__
    while parent is not object:
        classes.append(parent)
        parent = parent.__base__

    for cls in classes:
        for name, value in cls.__dict__.items():
            if callable(value) and not name.startswith('__'):
                names.add(name)

    return list(names)

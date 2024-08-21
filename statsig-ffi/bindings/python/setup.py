from setuptools import setup

setup(
    name='sigstat',
    version='0.1.2',
    license='ISC',
    packages=['sigstat'],
    package_data={
        'sigstat': ['*.so', '*.c', '*.o'],
    },
    include_package_data=True,
    python_requires='>=3.5',
)
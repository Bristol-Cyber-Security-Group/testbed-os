import enum
import logging
import harness_settings
from pathlib import Path
from urllib.request import urlretrieve

@enum.unique
class BaseOperatingSystem(enum.StrEnum):
    """
    This enum represents the range of operating systems that we are testing against. The corresponding value links to
    the latest version download of the disk image, ready to be used by libvirt.
    """
    # Ubuntu20_04 = "https://cloud-images.ubuntu.com/focal/current/focal-server-cloudimg-amd64.img"
    Ubuntu22_04 = "https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64.img"
    # Ubuntu24_04 = "https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img"

    def download_image(self):
        # download the image into the container
        if self.exists():
            return
        logging.info(f"Downloading image: {self}")
        urlretrieve(self, f"{harness_settings.workspace}/{self.get_filename()}")

    def exists(self) -> bool:
        # check if the image already exists on disk
        image = Path(f"{harness_settings.workspace}/{self.get_filename()}")
        if image.is_file():
            logging.info(f"{self.get_filename()} exists: {image}")
            return True
        return False

    def check_hash(self):
        # TODO - we should really evaluate the checksums of the existing image to the image online, to also check if
        #  potentially the image did not download properly - ubuntu provide the checksum list
        pass

    def get_filename(self) -> str:
        return self.split("/")[-1]


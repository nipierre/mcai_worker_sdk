import json
import logging
import os
import xml.etree.ElementTree as ET

def get_name():
    return "My python Media Worker"


def get_short_description():
    return "My python Media Worker"


def get_description():
    return """This is my long description
	over multilines
	"""


def get_version():
    return "0.0.3"


def get_parameters():
    return [
        {
            "identifier": "source_path",
            "label": "My parameter",
            "kind": ["string"],
            "required": True,
        },
        {
            "identifier": "destination_path",
            "label": "My array parameter",
            "kind": ["string"],
            "required": False,
        }
    ]


def init():
    '''
    Optional worker initialization function.
    '''

    print("Initialise Python worker...")

    log_level = os.environ.get('RUST_LOG', 'warning').upper()
    logging.basicConfig(format='[%(levelname)s] %(message)s', level=log_level)


def init_process(stream_handler, format_context, parameters):
    '''
    Function called before the media process (the "media" feature must be activated).
    '''
    logging.info("Initialise the media process...")
    logging.debug("Number of streams: %d", format_context.nb_streams)
    logging.debug("Message parameters: %s", parameters)

    # Here audio/video filters can be set to be applied on the worker input frames, using a simple python dict as follow.
    # Check the FFmpeg documentation to have more details on filters usage: https://ffmpeg.org/ffmpeg-filters.html
    stream_descriptors = []
    for stream in format_context.streams:
        if stream.stream_type == "AVMEDIA_TYPE_VIDEO":
            video_filters = [
                {
                    "name": "crop",
                    "label": "crop_filter",
                    "parameters": {
                       "out_w": "300",
                       "out_h": "200",
                       "x": "50",
                       "y": "50"
                    }
                }
            ]
            video_stream = stream_handler.new_video_stream(stream.index, video_filters)
            logging.info(f"Add video stream to process: {video_stream}")
            stream_descriptors.append(video_stream)

        if stream.stream_type == "AVMEDIA_TYPE_AUDIO":
            audio_filters = [
                {
                    "name": "aformat",
                    "parameters": {
                        "sample_rates": "16000",
                        "channel_layouts": "mono",
                        "sample_fmts": "s16"
                    }
                }
            ]
            audio_stream = stream_handler.new_audio_stream(stream.index, audio_filters)
            logging.info(f"Add audio stream to process: {audio_stream}")
            stream_descriptors.append(audio_stream)

        if stream.stream_type in ["AVMEDIA_TYPE_SUBTITLES", "AVMEDIA_TYPE_DATA"]:
            data_stream = stream_handler.new_data_stream(stream.index)
            logging.info(f"Add data stream to process: {data_stream}")
            stream_descriptors.append(data_stream)

    # returns a list of description of the streams to be processed
    return stream_descriptors


def process_frame(job_id, stream_index, frame):
    '''
    Process media frame (the "media" feature must be activated).
    '''
    data_length = 0
    for plane in range(0, len(frame.data)):
        data_length = data_length + len(frame.data[plane])

    if frame.width != 0 and frame.height != 0:
        logging.info(f"Job: {job_id} - Process video stream {stream_index} frame - PTS: {frame.pts}, image size: {frame.width}x{frame.height}, data length: {data_length}")
    else:
        logging.info(f"Job: {job_id} - Process audio stream {stream_index} frame - PTS: {frame.pts}, sample_rate: {frame.sample_rate}Hz, channels: {frame.channels}, nb_samples: {frame.nb_samples}, data length: {data_length}")

    # returns the process result as a JSON object (this is fully customisable)
    return { "status": "success" }


def process_ebu_ttml_live(job_id, stream_index, ttml_content):
    '''
    Process EBU TTML live content (the "media" feature must be activated).
    '''
    data_length = len(ttml_content)

    logging.info(f"Job: {job_id} - Process {data_length}-bytes EBU TTML live content from stream #{stream_index}: {ttml_content}")

    ttml_root = ET.fromstring(ttml_content)
    logging.debug(f"{ttml_root.tag}: {ttml_root.attrib}")

    subtitle = ""
    for body in ttml_root.findall('body'):
        subtitle += f"\t {body.tag}: {body.attrib}"
        for span in body.iterfind("div/p/span"):
            subtitle += f" - '{span.text}'"
    logging.info(subtitle)

    # returns the process result as a JSON object (this is fully customisable)
    return { "status": "success" }


def ending_process():
    '''
    Function called at the end of the media process (the "media" feature must be activated).
    '''
    logging.info("Ending Python worker process...")
